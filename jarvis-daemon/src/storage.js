import pg from 'pg';

const pool = new pg.Pool({
    host: 'localhost',
    database: 'jarvis-daemon',
    user: 'daemon',
    password: 'daemon',
    port: 54320,
});

export const findBuild = async function (build_key) {
    const client = await pool.connect()

    try {
        const res = await client.query('SELECT id, build_key FROM BUILD WHERE build_key=$1', [build_key]);

        if (res.rowCount === 0) {
            return null;
        }

        return res.rows[0];
    } finally {
        // Make sure to release the client before any error handling,
        // just in case the error handling itself throws an error.
        client.release()
    }
}

export const createBuild = async function (build_key, start_time) {
    const client = await pool.connect()

    try {
        await client.query('INSERT INTO BUILD (build_key, start_time) VALUES ($1, $2)', [build_key, start_time]);
    } finally {
        // Make sure to release the client before any error handling,
        // just in case the error handling itself throws an error.
        client.release()
    }
}

export const addEvent = async function (build_id, add_event) {
    const client = await pool.connect()

    try {
        await client.query('INSERT INTO BUILD_EVENT (build_id, name, code, time) VALUES ($1, $2, (SELECT code FROM EVENT_CODE where name=$3), $4)', [build_id, add_event.name, add_event.type, new Date()]);
    } finally {
        // Make sure to release the client before any error handling,
        // just in case the error handling itself throws an error.
        client.release()
    }
}
