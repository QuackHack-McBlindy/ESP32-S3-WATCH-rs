// GUI/TEXT
// DISPLAYS A LARGE TEXT ON THE DISPLAY 
// THE STRING IN QUESTION IS PROVIDED BY THE TEXT API ENDPOINT 

use crate::components::framebuffer::Framebuffer;

pub fn draw(fb: &mut Framebuffer) {
    // CLEAR SCREEN TO BLACK
    fb.buffer_mut().fill(0x0000);

    // READ THE DISPLAY STRING
    let string = critical_section::with(|cs| {
        crate::state::DISPLAY_STRING.borrow(cs).borrow().clone()
    });

    // DRAW THE TEXT (CENTERED)
    if let Some(string_str) = string.as_ref() {
        crate::gui::draw_text(fb, 150, 150, 106, string_str);
    }
}
