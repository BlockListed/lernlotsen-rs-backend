-- Add migration script here
DROP INDEX entries_timeslot_id_user_id;

CREATE INDEX entries_user_id_timeslot_id_index_idx ON entries (user_id, timeslot_id, index);