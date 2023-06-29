use serde::{Serialize, Deserialize};

#[derive(Deserialize, Debug)]
struct Input {
    name: String,
    age: u32,
    money: f32,
}

#[derive(Serialize, Debug)]
struct Output {
    name: String,
    taxes: f32,
}

#[repr(C)]
struct Slice {
    len: usize,
    ptr: *const u8,
}

#[no_mangle]
fn main(input_ptr: u32) -> Box<Slice> {
    // Read string from memory
    let input = unsafe {
        let len = *(input_ptr as *const u32);
        let bytes = (input_ptr + 4) as *const u8;
        let slice = core::slice::from_raw_parts(bytes, len as usize);
        rmp_serde::from_slice::<Input>(slice).unwrap()
    };
    
    println!("{input:?}");

    // Serialize output
    let output = Output {
        name: input.name,
        taxes: input.age as f32 * input.money * 0.10,
    };
    let output_bytes = rmp_serde::to_vec_named(&output).unwrap().leak();
    Box::new(Slice {
        len: output_bytes.len(),
        ptr: output_bytes.as_ptr(),
    })
}
