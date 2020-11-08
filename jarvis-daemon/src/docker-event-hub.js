import { getDockerEvents } from './docker-events';
import {findBuild, createBuild, addEvent} from "./storage";

async function getOrCreateBuild(data) {
    let build = null;

    if (data.Actor && data.Actor.Attributes) {
        const key = data.Actor.Attributes['build-id']
        build = await findBuild(key);

        if (build === null) {
            await createBuild(key, new Date());
            build = await findBuild(key);
        }
    }

    return build;
}

async function handleCreateEvent(data) {
    let build = await getOrCreateBuild(data);

    if (build !== null) {
        await addEvent(build.id, {
            type: 'build_container_started',
            name: data.Actor.Attributes['build-step']
        })
    }
}

async function handleDestroyEvent(data) {
    let build = await getOrCreateBuild(data);

    if (build !== null) {
        await addEvent(build.id, {
            type: 'build_container_stopped',
            name: data.Actor.Attributes['build-step']
        })
    }
}

export async function startDockerEventHub() {
    const dockerEvents = await getDockerEvents();

    dockerEvents.on('event', async (data) => {
        if (data.status === 'create') {
            await handleCreateEvent(data);
        }

        if (data.status === 'destroy') {
            await handleDestroyEvent(data);
        }
    });
}
