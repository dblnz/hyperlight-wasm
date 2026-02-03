#![allow(renamed_and_removed_lints)]
#![allow(unknown_lints)]
#![allow(unused_unit)]

use examples_common::get_wasm_module_path;

use bindings::hyperlight::bench::HandlerInterface;
use bindings::hyperlight::bench::handler_interface::Request;

extern crate alloc;
mod bindings {
    hyperlight_component_macro::host_bindgen!("../js_component_sample/wit/handler_wit.wasm");
}

pub struct State {}
impl State {
    pub fn new() -> Self {
        State {}
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl bindings::hyperlight::bench::HostInterface for State {
    fn r#print(&mut self, message: alloc::string::String) {
        println!("Logged from component: {message}");
    }
}

#[allow(refining_impl_trait)]
impl bindings::hyperlight::bench::HandlerWorldImports for State {
    type HostInterface = State;

    fn r#host_interface(&mut self) -> &mut Self {
        self
    }
}

fn main() {
    let state = State::new();
    let mut sb: hyperlight_wasm::ProtoWasmSandbox = hyperlight_wasm::SandboxBuilder::new()
        .with_guest_input_buffer_size(70000000)
        .with_guest_heap_size(200000000)
        .with_guest_stack_size(100000000)
        .build()
        .unwrap();
    let rt = bindings::register_host_functions(&mut sb, state);

    let sb = sb.load_runtime().unwrap();

    let mod_path = get_wasm_module_path("js_component_sample.aot").unwrap();
    let sb = sb.load_module(mod_path).unwrap();

    let mut wrapped = bindings::HandlerWorldSandbox { sb, rt };

    let instance =
        bindings::hyperlight::bench::HandlerWorldExports::handler_interface(&mut wrapped);
    let request = Request {
        uri: "Test Event".to_string(),
    };
    let result: Request = instance.handleevent(request);
    println!("Result is: {:?}", result);
}
