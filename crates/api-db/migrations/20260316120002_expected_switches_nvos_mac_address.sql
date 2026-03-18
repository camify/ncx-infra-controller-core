-- Add nvos_mac_address column to expected_switches table (NVOS host MAC, similar to bmc_mac_address).
ALTER TABLE expected_switches
    ADD COLUMN nvos_mac_address macaddr;
