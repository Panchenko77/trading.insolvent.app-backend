import * as zmq from 'zeromq';
import fs from 'fs';

import base58 from "bs58";
import {Drift} from "./drift";

// Generate a unique socket file path for the current user
const socketFilePath = process.argv[2] || `drift-zeromq.ipc`;
const address = `ipc://${socketFilePath}`;

const responder = new zmq.Router;
const drift: Drift = new Drift()

async function asyncEval(code: string) {
    return await eval(`(async (drift) => {
        ${code}
    })`)(drift)
}

async function startServer() {
    // Bind the socket to a specific address
    console.log(`Server is listening on ${address}`);
    await responder.bind(address);
    // Set file permissions to 700
    fs.chmodSync(socketFilePath, 0o700);
}

async function handleRequestRouter(request: Buffer[]) {
    // Assuming the first part of the message is the topic
    const client = request[0];
    const clientString = base58.encode(client);
    const requestId = request[1].toString();
    const command_ = request[2].toString();
    console.log(`Evaluating command#${clientString}-${requestId}: ${command_}`);
    // safety: we use an ipc socket that is only accessible to the current user
    try {
        let result
        if (command_.startsWith("{")) {
            const command = JSON.parse(command_);
            result = await drift.call(command.call, command.params);
        } else {
            result = await asyncEval(command_);
        }
        const resultString = JSON.stringify(result);
        console.log(`Result#${clientString}-${requestId}: ${resultString}`);

        await responder.send([client, requestId, resultString]);
    } catch (e: any) {
        console.error(`Error#${clientString}-${requestId}:`, e);
        const resultString = 'Error: ' + e.message;
        await responder.send([client, requestId, resultString]);
    }
}

async function startHandler() {
    console.log('[Ready] Drift ZeroMQ Server')

    // Handle incoming requests
    for await (const request of responder) {

        // no await
        handleRequestRouter(request);
    }
}

async function main() {
    await startServer();
    await drift.init();
    await startHandler();

}

main().catch((e) => {
    console.error('caught:', e);
    process.exit(-1);
});
