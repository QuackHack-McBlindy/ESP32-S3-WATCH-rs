// BASE/RNG

use core::cell::RefCell;
use critical_section::Mutex;
use esp_hal::rng::Rng;

static RNG: Mutex<RefCell<Option<Rng>>> = Mutex::new(RefCell::new(None));

pub fn init(rng: Rng) {
    critical_section::with(|cs| {
        RNG.borrow(cs).replace(Some(rng));
    });
}

getrandom::register_custom_getrandom!(custom_getrandom);

fn custom_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    critical_section::with(|cs| {
        let mut rng_ref = RNG.borrow(cs).borrow_mut();
        let rng = rng_ref.as_mut().ok_or(getrandom::Error::UNSUPPORTED)?;
        for chunk in buf.chunks_mut(4) {
            let val = rng.random();
            let len = chunk.len().min(4);
            chunk.copy_from_slice(&val.to_le_bytes()[..len]);
        }
        Ok(())
    })
}
