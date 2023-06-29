use anyhow::{anyhow as error, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use serde_json::{json, Value as Json};
use wasmer::{Instance, MemoryView, Module, Store, Value, ValueType, WasmSlice};
use wasmer_wasi::WasiState;

fn read<T: ValueType>(view: &MemoryView<'_>, offset: u64, length: u64) -> Result<Vec<T>> {
    Ok(WasmSlice::new(view, offset, length)?.read_to_vec()?)
}

struct Engine {
    store: wasmer::Store,
    instance: wasmer::Instance,
}

impl Engine {
    fn new(file: &str) -> Result<Self> {
        // Import the WASM file
        let mut store = Store::default();
        let module = Module::from_file(&store, file)?;

        // Create WASI environment
        let wasi_env = WasiState::new("engine").finalize(&mut store)?;
        let import_object = wasi_env.import_object(&mut store, &module)?;
        let instance = Instance::new(&mut store, &module, &import_object)?;

        // Pass memory reference to WASI
        let memory = instance.exports.get_memory("memory")?;
        wasi_env.data_mut(&mut store).set_memory(memory.clone());

        Ok(Self { store, instance })
    }

    fn run(&mut self, input: Json) -> Result<Json> {
        // Get function
        let function = self.instance.exports.get_function("main")?;
        let heap_start = 0x110000;

        // Serialize input into bytes
        let serialized = rmp_serde::to_vec(&input)?;
        let input_len = (serialized.len() as u32).to_le_bytes();
        let input_bytes = [&input_len[..], &serialized].concat();

        // Grow memory heap for input
        let memory = self.instance.exports.get_memory("memory")?;
        let pages = (input_bytes.len() / wasmer::WASM_PAGE_SIZE) + 1;
        memory.grow(&mut self.store, pages as u32)?;

        // Write bytes into memory
        let view = memory.view(&self.store);
        view.write(heap_start as u64, &input_bytes)?;

        // Pass pointer to start of our string
        let result = function.call(&mut self.store, &[Value::I32(heap_start)])?;

        // Deserialize data from pointer
        match result.first() {
            Some(Value::I32(pointer)) => {
                let view = memory.view(&self.store);
                let output_len = {
                    let bytes = read::<u8>(&view, *pointer as u64, 4)?;
                    bytes.as_slice().read_u32::<LittleEndian>()?
                };
                let output_ptr = {
                    let bytes = read::<u8>(&view, *pointer as u64 + 4, 4)?;
                    bytes.as_slice().read_u32::<LittleEndian>()?
                };
                let output_bytes = read::<u8>(&view, output_ptr as u64, output_len as u64)?;
                Ok(rmp_serde::from_read(output_bytes.as_slice())?)
            }
            _ => Err(error!(
                "Expected pointer to serialized data, got {result:?}"
            )),
        }
    }
}

fn main() -> Result<()> {
    let file = std::env::args().skip(1).next().expect("No file was passed");
    let mut engine = Engine::new(&file)?;
    let input = json!({
        "name": "James",
        "age": 50,
        "money": 50.60,
    });
    let output = engine.run(input)?;
    println!("Output {output}");
    Ok(())
}
