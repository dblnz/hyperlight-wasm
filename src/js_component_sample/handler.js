import { print } from "hyperlight:bench/host-interface"

export const handlerInterface = {
    handleevent(event) {
        print(`Handling event: ${event.uri}`);
        event.uri = '/redirected.html';
        return event;
    }
};
