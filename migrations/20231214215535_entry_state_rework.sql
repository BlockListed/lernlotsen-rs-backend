-- Add migration script here
CREATE TYPE "entry_state" AS ENUM (
	'success',
	'cancelledbystudents',
	'studentsmissing',
	'cancelledbytutor',
	'holidays',
	'other',
	'invaliddata'
);

CREATE TYPE "student_status" AS ENUM (
	'present',
	'pardoned',
	'missing'
);

CREATE TYPE "student_state" AS (
	student varchar(255),
	status student_status
);

ALTER TABLE "entries" ADD COLUMN "state_enum" entry_state;
ALTER TABLE "entries" ADD COLUMN "students" student_state[];

ALTER TABLE "entries" ALTER COLUMN "state" DROP NOT NULL;