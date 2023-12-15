-- Add migration script here
ALTER TABLE "entries" DROP COLUMN "students";

DROP TYPE "student_state";

CREATE TYPE "student_state" AS (
	student text,
	status student_status
);

ALTER TABLE "entries" ADD COLUMN "students" student_state[];