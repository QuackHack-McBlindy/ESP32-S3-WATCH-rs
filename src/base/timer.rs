// BASE/TIMER
// TIMER COUNTDOWN WITH EXTERNAL RESET ACCESS

// ─────────────────────────────────────────────────────────────────────────────
#[derive(Copy, Clone)]
enum TimerCommand {
    Start,
    Stop,
    Reset,
}

static TIMER_CMD: embassy_sync::channel::Channel<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    TimerCommand,
    1,
> = embassy_sync::channel::Channel::new();

// ─────────────────────────────────────────────────────────────────────────────
#[derive(Copy, Clone)]
pub struct TimerHandle;

impl TimerHandle {
    pub fn start(self) {
        let _ = TIMER_CMD.try_send(TimerCommand::Start);
    }

    pub fn stop(self) {
        let _ = TIMER_CMD.try_send(TimerCommand::Stop);
    }

    pub fn reset(self) {
        let _ = TIMER_CMD.try_send(TimerCommand::Reset);
    }
}

pub fn start() {
    let _ = TIMER_CMD.try_send(TimerCommand::Start);
}

pub fn stop() {
    let _ = TIMER_CMD.try_send(TimerCommand::Stop);
}

pub fn reset() {
    let _ = TIMER_CMD.try_send(TimerCommand::Reset);
}


// ─────────────────────────────────────────────────────────────────────────────
// TIMER COUNTDOWN TASK
#[embassy_executor::task]
pub async fn timer_task() {
    let mut enabled = false;
    let mut remaining = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);

    loop {
        defmt::info!("💤 ⌛ 💤");
        // IDLE - WAIT FOR START COMMAND
        while !enabled {
            match TIMER_CMD.receive().await {
                TimerCommand::Start => {
                    if !crate::load!(crate::state::BATTERY_USB_CONNECTED) {
                        enabled = true;
                        remaining = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
                        defmt::info!("💤 ⌛ ☑️ ({}s)", remaining);
                    } else { defmt::debug!("💤 ⌛ ❌ start ignored – USB connected!"); }
                }
                TimerCommand::Reset => {
                    // RESET WHILE IDLE
                    remaining = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
                    defmt::debug!("💤 ⌛ 🔄 {}s", remaining);
                }
                TimerCommand::Stop => {
                    // ALREADY IDLE
                }
            }
        }

        // ENABLED! COUNT DOWN!
        while enabled {
            let one_sec = embassy_time::Timer::after(embassy_time::Duration::from_secs(1));
            match embassy_futures::select::select(one_sec, TIMER_CMD.receive()).await {
                embassy_futures::select::Either::First(()) => {
                    if remaining > 0 {
                        remaining -= 1;
                        defmt::debug!("💤 ⌛: {}s left", remaining);
                    }

                    if remaining == 0 {
                        defmt::info!("💤 ⌛ 💥 FINISHED! Powering down...");
                        if !crate::load!(crate::state::BATTERY_USB_CONNECTED) { crate::deep_sleep_now(); }
                        enabled = false;
                        remaining = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
                    }
                }
                embassy_futures::select::Either::Second(cmd) => match cmd {
                    TimerCommand::Reset => {
                        remaining = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
                        defmt::info!("💤 ⌛ 🔄 {}s", remaining);
                    }
                    TimerCommand::Stop => {
                        defmt::info!("💤 ⌛ 🛑");
                        enabled = false;
                        remaining = crate::load!(crate::state::POWERDOWN_TIMEOUT_SECS);
                    }
                    TimerCommand::Start => {
                        defmt::debug!("start ignored timer already running");
                    }
                },
            }
        }
    }
}
