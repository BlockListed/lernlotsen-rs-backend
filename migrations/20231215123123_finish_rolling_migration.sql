-- Add migration script here
ALTER TABLE "entries" DROP COLUMN "state";

ALTER TABLE "entries" ALTER COLUMN "state_enum" SET NOT NULL;
ALTER TABLE "entries" ALTER COLUMN "students" SET NOT NULL;