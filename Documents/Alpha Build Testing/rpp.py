import datetime
import csv
import time

from scpi_driver.equipment.odp3122 import ODP3122
from scpi_driver.equipment.mso7104a import MSO7104A
from scpi_driver import ENABLE, DISABLE, VOLTAGE, CURRENT

ps = ODP3122('192.168.37.40')
osc = MSO7104A('192.168.37.41')

voltages_rpp = [2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24]
#voltages_rpp = [2, 4, 6]
voltage_lim = [3.3 + 1.8, 5 + 1.8]
test_index = 0

if (ps.ping):
    print (f'{ps.name} and {osc.name} are connected!')

    print(f'Disabling channel 1... {"Done!" if ps.set_output_channel(1, DISABLE) else "Failed!"}')

    print(f'Setting initial voltage values...' + 
          f'{"Done!" if ps.set_level_multiple(VOLTAGE, [0.01,None]) else "Failed!"}')
    print(f'Setting initial current values...' + 
          f'{"Done!" if ps.set_level_multiple(CURRENT, [.051,None]) else "Failed!"}')

    print('REVERSE POLARITY PROTECTION\n')
    input('Press any key to continue...')

    ps.change_mode(ODP3122.Mode.Remote)
    print(f'Setting channel 1 voltage limits...' +
          f'{"Done!" if ps.set_limit_multiple(VOLTAGE, [25, None]) else "Failed!"}')
    print(f'Setting channel 1 current limits...' +
          f'{"Done!" if ps.set_limit_multiple(CURRENT, [1.1, None]) else "Failed!"}')

    voltages_rail = []
    for i,v in enumerate(voltages_rpp):
        print(f'Collecting data...({i + 1}/{len(voltages_rpp)})\r', end='')
        ps.set_output_channel(1, DISABLE)
        ps.set_level_multiple(VOLTAGE, [v, None])
        ps.set_output_channel(1, ENABLE)
        time.sleep(2.0)
        voltages_rail.append(float(osc.send(':MEAS:VAV? CHAN1', 64)))
        time.sleep(0.5)

    print(f'Disabling channel 1... {"Done!" if ps.set_output_channel(1, DISABLE) else "Failed!"}')

    print(f'Setting final voltage values...' + 
          f'{"Done!" if ps.set_level_multiple(VOLTAGE, [0.01,None]) else "Failed!"}')
    
    print(f'Data collected: {voltages_rail}')

    now = datetime.datetime.now()
    filename = 'rpp-' + now.strftime("data-%Y.%m.%d_%H.%M.%S.csv")
    with (open(filename, 'x', newline='') as csvfile):
        #Create CSV writer used to transfer our data to the csv file.
        writer = csv.writer(csvfile, delimiter=',', quotechar='"', quoting=csv.QUOTE_MINIMAL)
        writer.writerow(['Input Voltage (V)', '3.3V Rail Voltage (V)'])

        for i in range(len(voltages_rail)):
            writer.writerow([voltages_rpp[i], voltages_rail[i]])

    print(f'Done! Data has been stored in {filename}.')