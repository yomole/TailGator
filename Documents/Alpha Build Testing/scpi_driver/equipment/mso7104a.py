from .scpi_equipment import scpi

class MSO7104A(scpi):
    '''
    The MSO7104a is an scpi-compatible 4 channel mixed signal Oscilloscope.
    '''

    def __init__(self, addr:str, port:int = 5025, timeout:float = 2.0) -> None:
        '''
        Initializes an MSO7104a instance and socket connection.
        
        Parameters
        ----------
        addr : str
            The IP address for the MSO7104a.
        port : int, optional
            The port for the MSO7104a (default is 3000).
        timeout : float, optional
            The amount of time, in seconds, that the socket connection should wait for a response
            after sending data.
        '''

        super().__init__(name='MSO7104A Oscilloscope', addr=addr, port=port, timeout=timeout)