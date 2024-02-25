import pytest
from unittest.mock import MagicMock
from scpi_driver.equipment.scpi_equipment import scpi
from scpi_driver.equipment.mso7104a import MSO7104A
from scpi_driver.equipment.odp3122 import ODP3122

def test_py():
    assert 1 == 1

def test_send_scpi(mocker):
    mock_socket = MagicMock()

    mock_socket.connect.return_value = None
    mock_socket.sendall.return_value = None
    mock_socket.recv.return_value = b'1'

    mocker.patch('scpi_driver.equipment.scpi_equipment.socket.socket', return_value=mock_socket)

    ps = ODP3122('192.168.37.40')

    assert ps.set_level_multiple(scpi.ElectricalQuantity.Voltage, [5, 3.3]) == True
    assert mock_socket.call_args[-1] == 'APP:VOLT 5,3.3'

    assert ps.name == 'ODP3122 Power Supply'


    with pytest.raises(scpi.InvalidElectricalQuantity):
        ps.set_level_multiple(scpi.ElectricalQuantity.Resistance, [37, 37])
