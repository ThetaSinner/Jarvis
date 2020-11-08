import EventEmitter from 'events';
import Docker from 'dockerode';

const docker = new Docker();

let docker_events = docker.getEvents({
    filters: {
        label: ["created-by=jarvis"],
        type: ["container"]
    }
}, undefined);

export async function getDockerEvents() {
    if (docker_events instanceof Promise) {
        docker_events = await docker_events;
    }

    const emitter = new EventEmitter();

    docker_events.on('data', data => {
        emitter.emit('event', JSON.parse(data.toString()));
    });

    docker_events.on('end', () => {
        console.log('events stream closed');
    });

    docker_events.on('error', e => {
        console.error('error on events stream', e);
    });

    return emitter;
}
