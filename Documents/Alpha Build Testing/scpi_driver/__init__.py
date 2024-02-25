from scpi_driver.equipment.scpi_equipment import scpi

ENABLE = scpi.ConfigBool.Enable
DISABLE = scpi.ConfigBool.Disable
VOLTAGE = scpi.ElectricalQuantity.Voltage
CURRENT = scpi.ElectricalQuantity.Current
POWER = scpi.ElectricalQuantity.Power
RESISTANCE = scpi.ElectricalQuantity.Resistance
INDUCTANCE = scpi.ElectricalQuantity.Inductance
FREQUENCY = scpi.ElectricalQuantity.Frequency
PERIOD = scpi.ElectricalQuantity.Period