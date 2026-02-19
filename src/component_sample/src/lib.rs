#[allow(warnings)]
#[rustfmt::skip]
mod bindings;

use bindings::exports::hyperlight::bench::handler_interface;
use bindings::exports::hyperlight::bench::handler_interface::Guest;
use bindings::hyperlight::bench::host_interface::print;

struct Component {}

impl Guest for Component {
    fn fib(n: i32) -> u64 {
        if n == 1 {
            return 1;
        } else if n <= 0 {
            return 0;
        }
        Self::fib(n - 1) + Self::fib(n - 2)
    }
    fn handleevent(event: handler_interface::Request) -> handler_interface::Request {
        print(&format!("Handling event: {}", event.uri));
        handler_interface::Request {
            uri: "/redirected.html".to_string(),
        }
    }
}

bindings::export!(Component with_types_in bindings);
