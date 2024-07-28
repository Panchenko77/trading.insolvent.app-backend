import zmq from 'zeromq';
import fs from 'fs';
// for testing purpose
// noinspection ES6UnusedImports
import fetch from 'node-fetch';


// Generate a unique socket file path for the current user
const socketFilePath = process.argv[2]
const address = `ipc://${socketFilePath}`;

async function asyncEval(code) {
    try {
        const result = await eval(`(async () => {
            ${code}
        })()`);
        return result;
    } catch (error) {
        throw error;
    }
}

async function startServer() {
    // Create a REP socket
    const responder = new zmq.Reply();
    // Bind the socket to a specific address
    console.log(`Server is listening on ${address}`);
    await responder.bind(address);
    // Set file permissions to 700
    fs.chmodSync(socketFilePath, 0o700);

    // Handle incoming requests
    for await (const [request] of responder) {
        // Assuming the first part of the message is the topic
        const command = request.toString();
        console.log(`Evaluating command: ${command}`);
        // safety: we use an ipc socket that is only accessible to the current user
        const result = await asyncEval(command);
        console.log(`Result: ${result}`);
        await responder.send(result);
    }
}

startServer().catch((err) => console.error(err));
// Cleanup the socket file on process exit
process.on('exit', () => {
    try {
        fs.unlinkSync(socketFilePath);
    } catch (err) {
        console.error(`Error cleaning up socket file: ${err.message}`);
    }
});