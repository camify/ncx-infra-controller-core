-- Add switch_reprovisioning_requested and firmware_upgrade_status columns to switches table.
-- switch_reprovisioning_requested: when set by an external entity, the state controller (when switch is Ready) transitions to ReProvisioning::Start.
-- firmware_upgrade_status: used during ReProvisioning (WaitFirmwareUpdateCompletion): Started, InProgress, Completed, Failed.
ALTER TABLE switches
    ADD COLUMN switch_reprovisioning_requested JSONB,
    ADD COLUMN firmware_upgrade_status JSONB;
