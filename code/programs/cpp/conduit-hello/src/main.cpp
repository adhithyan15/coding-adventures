// conduit-hello — entry point. Binds 127.0.0.1:<port> (default 3000) and serves
// in the foreground (the reactor dispatches handlers on this thread). Ctrl-C to
// stop.
//
//   ./conduit-hello          # port 3000
//   ./conduit-hello 8080     # or choose a port
//
// Then:  curl http://127.0.0.1:3000/hello/Ada
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <string>

#include "app.hpp"

int main(int argc, char** argv) {
    uint16_t port = 3000;
    if (argc > 1) {
        port = static_cast<uint16_t>(std::strtoul(argv[1], nullptr, 10));
    }
    try {
        conduit::Server server = make_app().bind("127.0.0.1", port);
        std::cout << "conduit-hello listening on http://127.0.0.1:" << server.localPort()
                  << "/  (Ctrl-C to stop)" << std::endl;
        server.serve();
    } catch (const std::exception& e) {
        std::cerr << "failed to start: " << e.what() << std::endl;
        return 1;
    }
    return 0;
}
