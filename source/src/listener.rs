use crate::{config::{parse_key, Config, Input}, service::Paths, Result};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::{collections::{HashMap, HashSet}, fs, os::fd::AsRawFd, path::Path, process::{Command, Stdio}, sync::mpsc, thread, time::{Duration, Instant}};
use tao::{event::{Event, StartCause}, event_loop::{ControlFlow, EventLoopBuilder}, platform::{macos::{ActivationPolicy, EventLoopExtMacOS}, run_return::EventLoopExtRunReturn}};

#[derive(Clone, Copy)]
enum Message { Key(GlobalHotKeyEvent), Exit }

pub fn bounded_switch(config_path: &Path, input: Input) -> Result<()> {
    let mut child = Command::new(std::env::current_exe()?)
        .arg("--config").arg(config_path).arg("send-input").arg(input.name())
        .stdin(Stdio::null()).spawn()?;
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() { Ok(()) } else { Err(format!("{} switch failed; see preceding error", input.name()).into()) };
        }
        if started.elapsed() >= Duration::from_secs(4) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Monitor did not respond within four seconds; listener remains available.".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn run(config_path: &Path, config: Config, paths: &Paths, check: bool, listen_seconds: Option<u64>) -> Result<()> {
    let mut lock_file = None;
    let live = !check && listen_seconds.is_none();
    if live {
        // Advisory locking survives crashes without leaving a stale process lock.
        fs::create_dir_all(&paths.data)?;
        let file = fs::OpenOptions::new().create(true).truncate(false).read(true).write(true).open(paths.data.join("listener.lock"))?;
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Monitor Keys is already running. Use `restart` to reload its configuration.".into());
        }
        lock_file = Some(file);
        if paths.ready.exists() { fs::remove_file(&paths.ready)?; }
    }
    let mut event_loop = EventLoopBuilder::<Message>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Prohibited);
    event_loop.set_dock_visibility(false);
    event_loop.set_activate_ignoring_other_apps(false);
    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| { let _ = proxy.send_event(Message::Key(event)); }));
    if let Some(seconds) = listen_seconds {
        let proxy = event_loop.create_proxy();
        thread::spawn(move || { thread::sleep(Duration::from_secs(seconds)); let _ = proxy.send_event(Message::Exit); });
    }
    // One worker serializes writes and bounds queued requests. The event loop never waits on DDC.
    let (sender, receiver) = mpsc::sync_channel::<Input>(1);
    let config_file = config_path.to_path_buf();
    if live {
        thread::spawn(move || {
            while let Ok(input) = receiver.recv() {
                if let Err(error) = bounded_switch(&config_file, input) { eprintln!("{error}"); }
            }
        });
    }
    let manager = GlobalHotKeyManager::new()?;
    let mut bindings = HashMap::new();
    let mut registered = Vec::new();
    let mut failure: Option<String> = None;
    let mut pressed = HashSet::new();
    let mut last_action: Option<Instant> = None;
    event_loop.run_return(|event, _, control| {
        *control = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                for input in Input::ALL {
                    let key = parse_key(&config.binding(input).key).expect("validated config");
                    if let Err(error) = manager.register(key) {
                        failure = Some(format!("Cannot register {} for {}: {error}. Another app or a macOS shortcut may own this key. Change the binding or release the conflict.", config.binding(input).key, input.name()));
                        *control = ControlFlow::Exit;
                        return;
                    }
                    registered.push(key);
                    bindings.insert(key.id(), input);
                }
                let mapping = Input::ALL.map(|i| format!("{} → {}", config.binding(i).key, i.name())).join("\n");
                println!("Global F-key shortcuts registered.\n{mapping}");
                if live {
                    if let Err(error) = fs::write(&paths.ready, format!("{}\n{mapping}\n", std::process::id())) {
                        failure = Some(error.to_string());
                        *control = ControlFlow::Exit;
                    }
                } else if check { *control = ControlFlow::Exit; }
                else { println!("Listening only — no monitor commands. Switch to another app and press your F-keys."); }
            }
            Event::UserEvent(Message::Exit) => *control = ControlFlow::Exit,
            Event::UserEvent(Message::Key(event)) => {
                if event.state == HotKeyState::Released { pressed.remove(&event.id); return; }
                if !pressed.insert(event.id) { return; }
                if let Some(input) = bindings.get(&event.id) {
                    if !live { println!("Received {} → {}", config.binding(*input).key, input.name()); return; }
                    if last_action.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) { return; }
                    last_action = Some(Instant::now());
                    if sender.try_send(*input).is_err() { eprintln!("Monitor busy; ignored extra keypress."); }
                }
            }
            _ => {}
        }
    });
    for key in registered { let _ = manager.unregister(key); }
    if live { let _ = fs::remove_file(&paths.ready); }
    drop(lock_file);
    if let Some(error) = failure { Err(error.into()) } else { Ok(()) }
}
