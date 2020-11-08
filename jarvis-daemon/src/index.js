import "core-js/stable";
import "regenerator-runtime/runtime";

const WebSocket = require('ws');
import './disk-usage'

import {startDockerEventHub} from "./docker-event-hub";

(async () => {
    await startDockerEventHub();
})();

const wsServer = new WebSocket.Server({
    port: 9001,
    perMessageDeflate: {
        zlibDeflateOptions: {
            // See zlib defaults.
            chunkSize: 1024,
            memLevel: 7,
            level: 3
        },
        zlibInflateOptions: {
            chunkSize: 10 * 1024
        },
        // Other options settable:
        clientNoContextTakeover: true, // Defaults to negotiated value.
        serverNoContextTakeover: true, // Defaults to negotiated value.
        serverMaxWindowBits: 10, // Defaults to negotiated value.
        // Below options specified as default values.
        concurrencyLimit: 10, // Limits zlib concurrency for perf.
        threshold: 1024 // Size (in bytes) below which messages
        // should not be compressed.
    }
});

wsServer.on('connection', async (ws) => {
    ws.on('message', (message) => {
        console.log('received: %s', message);
    });

    ws.on('close', () => {
        // diskUsageSubscription.unsubscribe();
    });
});
