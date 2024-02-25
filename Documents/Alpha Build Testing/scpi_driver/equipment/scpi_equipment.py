import socket
from enum import Enum, auto
import time

class scpi:
    '''
    The scpi class provides common, useful functions for lab equipment and provides an easy way to
    initialize a connection to lab equipment.

    Attributes
    ----------
    name : str
        the name given to the lab equipment's scpi connection. This is mostly used for error
        messages.
    sckt : socket
        socket object that is used for establishing SCPI connections and transcieving data.
    current_channel : int
        The currently selected channel used for channel-specific commands. The default value is 1.
    num_channels : int
        The number of channels available in the lab equipment. The default value is 1.
    quantities : list[scpi.ElectricalQuantity]
        The available quantities to measure or output on the equipment. The default value is None.
    '''

    # --------------------------------------- ENUMERATIONS --------------------------------------- #

    class ConfigBool(Enum):
        '''
        Enumeration used to clarify parameters to enable or disable functions of lab equipment.
        '''
        Disable = 0
        Enable = 1

    class ElectricalQuantity(Enum):
        '''
        Generic enumeration class that defines different types of measurable or configurable
        electrical quantities along with their common abbreviations in SCPI commands.
        '''

        Voltage = auto()
        Current = auto()
        Power = auto()
        Resistance = auto()
        Inductance = auto()
        Frequency = auto()
        Period = auto()

        @property
        def abbreviation(self) -> str:
            '''
            Returns the common abbreviation of the electrical quantity used in SCPI commands.
            '''
            abbreviations = {
                scpi.ElectricalQuantity.Voltage: 'VOLT',
                scpi.ElectricalQuantity.Current: 'CURR',
                scpi.ElectricalQuantity.Power: 'POW',
                scpi.ElectricalQuantity.Resistance: 'RES',
                scpi.ElectricalQuantity.Inductance: 'IND',
                scpi.ElectricalQuantity.Frequency: 'FREQ',
                scpi.ElectricalQuantity.Period: 'PER',
            }

            return abbreviations.get(self)

    # --------------------------------------- ENUMERATIONS --------------------------------------- #

    # ------------------------------------------ ERRORS ------------------------------------------ #
        
    class InvalidElectricalQuantity(Exception):
        '''Handles errors related to invalid electrical quantities.'''
        def __init__(self, name:str, quantities:list):
            super().__init__(f'Electrical quantity provided for \"{name}\" is invalid.' +
                             f'\nAvailable types are: {quantities}')

    # ------------------------------------------ ERRORS ------------------------------------------ #

    # -------------------------------------- CLASS FUNCTIONS ------------------------------------- #

    def __init__(self, name:str, addr:str, port:int = 5555, timeout:float = 2.0, num_channels = 1, quantities:list = None) -> None:
        '''
        Initializes a SCPI object for the given lab equipment and initializes the socket connection.

        Parameters
        ----------
        name : str
            The name for the lab equipment's SCPI connection.
        addr : str
            The IP Address for the lab equipment.
        port : int, optional
            The port for the lab equipment (default is 5555).
        timeout : float, optional
            The amount of time, in seconds, that the socket connection should wait for a response
            after sending data.
        quantities : list[scpi.ElectricalQuantity]
            The available quantities to measure or output on the ODP3122. The default is None.
        '''

        try: 
            self.sckt = socket.create_connection((addr, port))
            self.sckt.settimeout(2.0)
        except Exception as e:
            print(f"socket connection for \"{name}\" could not be made:\n{e}")
        
        self.name = name
        self.channel = 1
        self.num_channels = num_channels
        self.quantities = quantities

    def send(tool, msg:str, size:int = 0):
        '''
        Sends a message to the lab equipment using the SCPI object.

        Parameters
        ----------
        tool : scpi
            The equipment's SCPI object used to transmit the data.
        msg : str
            The SCPI message to send. This message should NOT have a terminating new line charater.
        size : int, optional
            The expected size of the response (default is 0).
        
        Returns
        -------
        Union[str, None]
            The data in string format if there is a response. Otherwise, it returns None.
        '''

        tool.sckt.send((msg+"\n").encode('utf-8'))
        try:
            data = tool.sckt.recv(size)
            time.sleep(0.5)
            return data.decode('utf-8')
        except TimeoutError:
            return None
        
    def ping (tool) -> bool:
        '''
        "Pings" a piece of lab equipment by checking that its response to *IDN? is not blank.

        Parameters
        ----------
        tool : scpi
            The equipment's SCPI object used to transmit the data.
        '''

        return (True if tool.sckt.send(("*IDN").encode('utf-8')) != None else False)
    
    def reset(tool) -> None:
        '''
        Resets a piece of lab equipment to factory settings.

        Parameters
        ----------
        tool : scpi
            The equipment's SCPI object used to transmit the data.
        '''

        tool.sckt.send(("*RST"))


    # -------------------------------------- CLASS FUNCTIONS ------------------------------------- #