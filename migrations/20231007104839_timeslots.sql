-- Add migration script here
CREATE TYPE student AS (
	name varchar(255)
);

CREATE TYPE timeslot_time AS (
	start_time	time,
	end_time	time
);

CREATE TYPE timeslot_range AS (
	start_date	date,
	end_date	date
);

CREATE TABLE timeslots (
	id uuid PRIMARY KEY,
	user_id varchar(255) NOT NULL,
	subject varchar(127) NOT NULL,
	students student[] NOT NULL,
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