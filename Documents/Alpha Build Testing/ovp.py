import datetime
import csv
import time

from scpi_driver.equipment.odp3122 import ODP3122
from scpi_driver.equipment.mso7104a import MSO7104A
from scpi_driver import ENABLE, DISABLE, VOLTAGE, CURRENT

ps = ODP3122('192.168.37.40')
osc = MSO7104A('192.168.37.41')

voltages_ovp = [2.7, 2.9, 3.1, 3.3, 3.5, 3.7, 3.9, 4.1, 4.3, 4.5]

if (ps.ping):
    print (f'{ps.name} and {osc.name} are connected!')

    print(f'Disabling channel 1... {"Done!" if ps.set_output_channel(1, DISABLE) else "Failed!"}')

    ps.change_mode(ODP3122.Mode.Remote)
    print(f'Setting channel 1 voltage limits...' +
          f'{"Done!" if ps.set_limit_multiple(VOLTAGE, [25, None]) else "Failed!"}')
    print(f'Setting channel 1 current limits...' +
          f'{"Done!" if ps.set_limit_multiple(CURRENT, [5.5, None]) else "Failed!"}')

    print(f'Setting initial voltage values...' + 
          f'{"Done!" if ps.set_level_multiple(VOLTAGE, [0.01,None]) else "Failed!"}')
    print(f'Setting initial current values...' + 
          f'{"Done!" if ps.set_level_multiple(CURRENT, [5,None]) else "Failed!"}')

    print('OVERVOLTAGE PROTECTION\n')
    input('Press any key to continue...')

    voltages_rail = []
    voltages_protection = []
    for i,v in enumerate(voltages_ovp):
        print(f'Collecting data...({i + 1}/{len(voltages_ovp)})\r', end='')
        ps.set_output_channel(1, DISABLE)
        ps.set_level_multiple(VOLTAGE, [v, None])
        ps.set_output_channel(1, ENABLE)
        time.sleep(2.0)
        voltages_rail.append(float(osc.send(':MEAS:VAV? CHAN1', 64)))
        voltages_protection.append(float(osc.send(':MEAS:VAV? CHAN2', 64)))
        time.sleep(0.5)

    print(f'Disabling channel 1... {"Done!" if ps.set_output_channel(1, DISABLE) else "Failed!"}')

    print(f'Setting final voltage values...' + 
          f'{"Done!" if ps.set_level_multiple(VOLTAGE, [0.01,None]) else "Failed!"}')
    
    print(f'Data collected: {voltages_rail}')

    now = datetime.datetime.now()
    filename = 'ovp-' + now.strftime("data-%Y.%m.%d_%H.%M.%S.csv")
    with (open(filename, 'x', newline='') as csvfile):
        #Create CSV writer used to transfer our data to the csv file.
        writer = csv.writer(csvfile, delimiter=',', quotechar='"', quoting=csv.QUOTE_MINIMAL)
        writer.writerow(['Input Voltage (V)', '3.3V Rail Voltage (V)', '3.3V OVP Circuit Voltage (V)'])

        for i in range(len(voltages_rail)):
            writer.writerow([voltages_ovp[i], voltages_rail[i], voltages_protection[i]])

    print(f'Done! Data has been stored in {filename}.')