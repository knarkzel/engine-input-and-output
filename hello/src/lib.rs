#[no_mangle]
fn main(input_ptr: u32) {
    // Read string from memory
    let input = unsafe {
        let len = *(input_ptr as *const u32);
        let bytes = (input_ptr + 4) as *const u8;
        let slice = core::slice::from_raw_parts(bytes, len as usize);
        core::str::from_utf8_unchecked(slice)
    };
    
    println!("{input}");
}
