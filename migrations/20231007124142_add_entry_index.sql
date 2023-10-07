-- Add migration script here
CREATE INDEX entries_timeslot_id_user_id ON entries (timeslot_id, user_id, index);
CREATE INDEX timeslots_user_id ON timeslots (user_id);