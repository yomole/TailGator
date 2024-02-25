import datetime
import csv
import time

from scpi_driver.equipment.odp3122 import ODP3122
from scpi_driver.equipment.mso7104a import MSO7104A
from scpi_driver import ENABLE, DISABLE, VOLTAGE, CURRENT

ps = ODP3122('192.168.37.40')
osc = MSO7104A('192.168.37.41')

v_ovp = 4.1

if (ps.ping):
    print (f'{ps.name} and {osc.name} are connected!')

    print(f'Disabling channel 1... {"Done!" if ps.set_output_channel(1, DISABLE) else "Failed!"}')

    ps.change_mode(ODP3122.Mode.Remote)
    print(f'Setting channel 1 voltage limits...' +
          f'{"Done!" if ps.set_limit_multiple(VOLTAGE, [25, None]) else "Failed!"}')
    print(f'Setting channel 1 current limits...' +
          f'{"Done!" if ps.set_limit_multiple(CURRENT, [5.5, None]) else "Failed!"}')

    print(f'Setting initial voltage values...' + 
          f'{"Done!" if ps.set_level_multiple(VOLTAGE, [v_ovp,None]) else "Failed!"}')
    print(f'Setting initial current values...' + 
          f'{"Done!" if ps.set_level_multiple(CURRENT, [5,None]) else "Failed!"}')

    print('OVERVOLTAGE PROTECTION TIME TO ACTIVATE\n')
    input('Press any key to continue...')

    ps.set_output_channel(1, ENABLE)
    start = time.time()

    while(ps.get_level_multiple(CURRENT)[0] != 0):
        current = time.time()
        print(f'Waiting for fuse to blow... ({current - start} seconds so far!)', end='\r')

    print(f'Disabling channel 1... {"Done!" if ps.set_output_channel(1, DISABLE) else "Failed!"}')

    print(f'Setting final voltage values...' + 
          f'{"Done!" if ps.set_level_multiple(VOLTAGE, [0.01,None]) else "Failed!"}')
    
    print(f'Fuse blew after {current - start} seconds of overvoltage protection.')