-- Add migration script here
CREATE TYPE timeslot_time AS (
	beginning	time,
	finish	time
);

CREATE TYPE timeslot_range AS (
	beginning	date,
	finish	date
);

CREATE TABLE timeslots (
	id uuid PRIMARY KEY,
	user_id varchar(255) NOT NULL,
	subject varchar(127) NOT NULL,
	students varchar(255)[] NOT NULL,
	time timeslot_time NOT NULL,
	timerange timeslot_range NOT NULL,
	timezone varchar(127) NOT NULL
);

CREATE table entries (
	id uuid PRIMARY KEY,
	user_id varchar(255) NOT NULL,
	index integer NOT NULL,
	timeslot_id uuid NOT NULL REFERENCES timeslots(id) ON DELETE CASCADE,
	state jsonb NOT NULL
);