import { print } from "hyperlight:bench/host-interface"

export const handlerInterface = {
    fib(n) {
        if (n <= 1) return n;
        let a = 0, b = 1;
        for (let i = 2; i <= n; i++) {
            [a, b] = [b, a + b];
        }
        return b;
    },

    handleevent(event) {
        print(`Handling event: ${event.uri}`);
        event.uri = '/redirected.html';
        return event;
    }
};
