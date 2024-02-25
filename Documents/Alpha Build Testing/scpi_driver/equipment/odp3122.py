from .scpi_equipment import scpi
from enum import Enum

class ODP3122(scpi):
    '''
    The ODP3122 is an scpi-compatible 2 channel power supply with the following specs:
    
    CH1:
        - 0V -> 30V Voltage Output, up to 31V OVP
        - 0A -> 12A Current Output, up to 12.1A OCP

    CH2:
        - 0V -> 6V Voltage Output, up to 6.6V OVP
        - 0A -> 3A Current Output, up to 3.1A OCP
    '''

    # --------------------------------------- ENUMERATIONS --------------------------------------- #

    class Mode(Enum):
        '''
        Defines an enumeration for power supply system modes along with their SCPI codes.
        - Local Mode allows interaction with the power supply through SCPI and the front panel buttons.
        - Remote Mode only allows interaction through SCPI by locking all buttons besides the
        keylock button.

        See pg. 3, 5 of the ODP Series Programming Manual for more information.
        '''
        Local = 'SYST:LOC'
        Remote = 'SYST:REM'

    # --------------------------------------- ENUMERATIONS --------------------------------------- #

    # -------------------------------------- CLASS FUNCTIONS ------------------------------------- #

    def __init__(self, addr:str, port:int = 3000, timeout:float = 2.0) -> None:
        '''
        Initializes an ODP3122 instance and socket connection.
        
        Parameters
        ----------
        addr : str
            The IP address for the ODP3122.
        port : int, optional
            The port for the ODP3122 (default is 3000).
        timeout : float, optional
            The amount of time, in seconds, that the socket connection should wait for a response
            after sending data.
        '''

        super().__init__(name='ODP3122 Power Supply', addr=addr, port=port, timeout=timeout,
                         num_channels= 2, quantities=[scpi.ElectricalQuantity.Voltage, 
                                                      scpi.ElectricalQuantity.Current, 
                                                      scpi.ElectricalQuantity.Power])

    def change_mode(self, mode:Mode):
        '''
        Changes the interaction mode of the power supply.

        Parameters
        ----------
        mode : ODP3122.Mode
            The interaction mode of the power supply.
        '''

        self.send(mode.value)
    
    def select_channel(self, channel:int) -> bool:
        '''
        Sets the currently selected channel and checks if the command was successful.
        '''

        if (channel > 0 and channel <= self.num_channels):
            self.send('INST CH' + str(channel))
        else:
            raise ValueError(f'Channel {channel} is not a valid channel for {self.name}.')
        
        if (self.get_channel() == 'CH' + str(channel)):
            return True 
        else:
            return False


    def get_current_channel(self) -> int:
        '''
        Returns the currently selected channel.

        Returns
        -------
        int
        '''

        return self.send('INST?', 64)

    def set_level_multiple(self, type:scpi.ElectricalQuantity, values : list) -> bool:
        '''
        Sets the voltage or current output for multiple channels without changing the selected
        channel.

        Parameters
        ----------
        type : scpi.ElectricalQuantity
            The type of quanitity to adjust on the channels.
        values : list
            The values to set the outputs to. Values greater than the number of channels will be
            ignored.
        '''

        base_cmd = 'APP:'

        if type in self.quantities:
            base_cmd += (type.abbreviation + ' ')
        else:
            raise self.InvalidElectricalQuantity(self.name, self.quantities)

        cmd = base_cmd

        initial_values = self.get_level_multiple(type)
        for i,v in enumerate(values):
            if i > self.num_channels:
                break
            if v == None:
                cmd += (str(initial_values[i]) + ',')
            else:
                cmd += (str(v) + ',')
        
        cmd = cmd[:-1]

        self.send(cmd)

        actual_values = self.get_level_multiple(type)
        for i,v in enumerate(actual_values):
            if (values[i] != v and values[i] != None):
                return False
            
        return True

    def get_level_multiple(self, type:scpi.ElectricalQuantity) -> list:
        '''
        Returns the output values for a specified quantity.

        Parameters
        ----------
        type : scpi.ElectricalQuantity
        
        '''
        cmd = 'APP:'

        if type in self.quantities and type != scpi.ElectricalQuantity.Power:
            cmd += (type.abbreviation + '?')
        else:
            raise self.InvalidElectricalQuantity(self.name, [scpi.ElectricalQuantity.Voltage])

        data = self.send(cmd, 64)
        data = data.split(',')

        for i in range(len(data)):
            data[i] = data[i].strip()
            data[i] = data[i].replace('\n', '')
            data[i] = float(data[i])

        return data

    def set_limit_multiple(self, type:scpi.ElectricalQuantity, limits:list) -> bool:
        '''
        Sets the voltage or current limit for multiple channels without changing the selected
        channel.

        Parameters
        ----------
        type : scpi.ElectricalQuantity
            The type of quanitity to adjust on the channels.
        values : list
            The values to set the outputs to. Values greater than the number of channels will be
            ignored.
        '''

        base_cmd = ''

        if type in self.quantities:
            base_cmd += (type.abbreviation + ':LIM:ALL ')
        else:
            raise self.InvalidElectricalQuantity(self.name, self.quantities)

        cmd = base_cmd


        initial_values = self.get_limit_multiple(type)
        for i,v in enumerate(limits):
            if i > self.num_channels:
                break
            if v == None:
                cmd += (str(initial_values[i]) + ',')
            else:
                cmd += (str(v) + ',')
        
        cmd = cmd[:-1]

        self.send(cmd)

        actual_limits = self.get_limit_multiple(type)
        for i,v in enumerate(actual_limits):
            if (limits[i] != v and limits[i] != None):
                print(f'Expected {limits[i]}, got {v}')
                return False
            
        return True

    def get_limit_multiple(self, type:scpi.ElectricalQuantity) -> list:
        '''
        Returns the output values for a specified quantity.

        Parameters
        ----------
        type : scpi.ElectricalQuantity
        
        '''
        cmd = ''

        if type in self.quantities and type != scpi.ElectricalQuantity.Power:
            cmd += (type.abbreviation + ':LIM:ALL?')
        else:
            raise self.InvalidElectricalQuantity(self.name, [scpi.ElectricalQuantity.Voltage])

        data = self.send(cmd, 64)
        data = data.split(',')

        for i in range(len(data)):
            data[i] = data[i].strip()
            data[i] = data[i].replace('\n', '')
            data[i] = float(data[i])

        return data

    def measure_channel(self, type:scpi.ElectricalQuantity, channel:int) -> float:
        '''
        Measures a certain quantity of a specified channel.

        Parameters
        ----------
        type : scpi.ElectricalQuantity
            The quantity to measure.
        channel : int
            The channel to measure the quantity on.

        Returns
        -------
        float
        '''

        self.set_channel(channel)

        cmd = 'MEAS:'

        if type in self.quantities:
            cmd += (type.abbreviation + '?')
        else:
            raise self.InvalidElectricalQuantity(self.name, [scpi.ElectricalQuantity.Voltage])

        return float(self.send(cmd, 64))

    def get_status(self) -> list:
        '''
        Gets the status of all channels.
        
        Returns
        -------
        list[bool]
        '''

        data = self.send('CHAN:OUTP:ALL?', 64)
        data = data.split(',')

        for i in range(len(data)):
            data[i] = int(data[i])
        
        return data

    def set_output_channel(self, channel:int, mode:scpi.ConfigBool) -> bool:
        '''
        Enables a specified channel. Checks if the channel was actually enabled.
        
        Parameters
        ----------
        channel : int
            The channel to enable.
        mode: scpi.ConfigBool
            Whether to enable or disable the channel.

        Returns
        -------
        bool
        '''

        if (channel > 0 and channel <= self.num_channels):
            status = self.get_status()

            cmd = status
            cmd[channel - 1] = mode.value
            self.send('CHAN:OUTP:ALL ' + ','.join(map(str, cmd)))

            status = self.get_status()
            for i,v in enumerate(status):
                if (cmd[i] != v):
                    return False

            return True

        else:
            raise ValueError(f'Channel {channel} is not a valid channel for {self.name}.')


    # -------------------------------------- CLASS FUNCTIONS ------------------------------------- #