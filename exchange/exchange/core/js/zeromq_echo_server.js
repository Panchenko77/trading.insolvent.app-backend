import zmq from 'zeromq';

const address = 'tcp://127.0.0.1:5555'

async function startServer() {
    // Create a REP socket
    const responder = new zmq.Reply();
    // Bind the socket to a specific address
    await responder.bind(address);
    console.log(`Server is listening on ${address}`);

    // Handle incoming requests
    for await (const [request] of responder) {
        // Assuming the first part of the message is the topic
        const topic = request.toString();
        console.log(`Received request: ${topic}`);
        console.log(`Responding: ${topic}`);
        await responder.send(topic);
    }
}

startServer().catch((err) => console.error(err));
