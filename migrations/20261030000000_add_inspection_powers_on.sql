ALTER TABLE device_inspections ADD COLUMN powers_on boolean;

COMMENT ON COLUMN device_inspections.powers_on IS
    'Whether the device powered on during this inspection. True means it did, false means an attempted power-on failed, and null means not checked. Independent of component results; never inherited from older inspections.';
