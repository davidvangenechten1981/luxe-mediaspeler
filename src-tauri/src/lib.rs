mod launch;
mod logging;
mod shell_integration;
#[cfg(windows)]
mod winvideo;

#[cfg(feature = "core")]
mod app;
#[cfg(feature = "core")]
mod commands;
#[cfg(feature = "core")]
mod power;
#[cfg(feature = "core")]
mod sink;
#[cfg(all(feature = "core", windows))]
mod smtc;

pub fn run() {
    #[cfg(feature = "core")]
    app::run();
    #[cfg(not(feature = "core"))]
    panic!("Dit programma is zonder de kern gebouwd (alleen voor typecontrole)");
}
