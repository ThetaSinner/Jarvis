CREATE TABLE BUILD (
    id SERIAL PRIMARY KEY,
    build_key VARCHAR(24) UNIQUE NOT NULL,
    start_time TIMESTAMP,
    end_time TIMESTAMP
);

CREATE TABLE EVENT_CODE (
    code SERIAL PRIMARY KEY,
    name VARCHAR(75)
);

CREATE TABLE BUILD_EVENT (
    id SERIAL PRIMARY KEY,
    build_id SERIAL NOT NULL,

    name VARCHAR(255) NOT NULL,
    code INT NOT NULL,
    time TIMESTAMP NOT NULL,

    FOREIGN KEY (build_id) REFERENCES BUILD(id),
    FOREIGN KEY (code) REFERENCES EVENT_CODE(code)
);

INSERT INTO EVENT_CODE (code, name) VALUES
    (10001, 'build_container_started'),
    (10002, 'build_container_stopped');
