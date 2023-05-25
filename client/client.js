import WebSocket from 'ws';
import readline from 'readline';

const socket = new WebSocket("ws://localhost:3012")

socket.on('open', () => {
    socket.send("oh boy here we go connecting again");
});

socket.on('message', (message) => {
    console.log('%s', message);
})

socket.on('open', () => {
        readline.createInterface(process.stdin).on('line', (input) => { 
            socket.send(input);
        })
})
