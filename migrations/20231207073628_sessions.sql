-- Add migration script here
CREATE TYPE session_status AS ENUM ('initiated', 'authenticated');

CREATE TABLE sessions (
	id uuid PRIMARY KEY,
	authenticated session_status NOT NULL,
	nonce varchar(127) NOT NULL,
	user_id varchar(255),
	expires timestamp with time zone NOT NULL
);