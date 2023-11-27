# TailGator

# Introduction

The TailGator Interconnect System (TGIS) project aims to produce a modular connection system and standard for easily connecting, replacing, and upgrading underwater robotics hardware in space-constrained robotics designs. 
Several other standards exist for modular electronics hardware, including:

- [Adafruit Feather Specification](https://learn.adafruit.com/adafruit-feather/feather-specification)
- [Sparkfun MicroMod Specification](https://www.sparkfun.com/micromod)
- [Arduino Shield Specificaton](https://learn.sparkfun.com/tutorials/arduino-shields-v2)
- [PCI-e Specification](https://pcisig.com/)

However, no single standard contains the pin density, physical dimensions, signal integrity, power delivery, and cost effectiveness we are looking for. The TailGator Interconnect System aims to solve this problem for the Machine Intelligence Laboratories' [SubjuGator 9 Autonomous Underwater Vehicle](http://subjugator.org/?page_id=3390) platform while making the design open source to help the robotics community.

# Electrical Specifications

The current design of TGIS can supply:
- 3.3V @ 4A (13.3W)
- 5V @ 4A (20W)

Reserved lines may be used to supplement the 3.3V or 5V lines. Board designs may also include a 5V to 3.3V LDO regulator to suppliment the 3.3V line.

# Pinout

![image](Documents/Images/Pinout-11-26-23.jpg)

# Simulations

These are LTSpice simulations for some parts of the circuits we designed. Libraries used are included.

- `3V3_Protection.asc` is an LTSpice simulation of the 3.3V protection circuits. This circuit contains reverse polarity protection, overcurrent protection, and overvoltage protection.
- `5V_Protection.asc` is an LTSpice simulation of the 5V protection circuits. This circuit contains reverse polarity protection, overcurrent protection, and overvoltage protection.

## Libraries

The libraries in this folder are used for some of the components in the LTSpice circuits.

- `fuse2.lib` is a library of fuses compiled by user aurvii based on a fuse model by Helmut Sennewald. It can be found in the [Official LTSpice Support Group](https://groups.io/g/LTspice).
- `st_standard_sensitve_scr` is a library of sensitive and standard SCRs/Thyristors by STMicroelectronics. It can be found [here](https://www.st.com/resource/en/spice_model/standard_sensitive_scr_pspice.zip).
- `Zener_DiodesInc.lib` is a modified version of this [zener diodes library](https://www.diodes.com/productcollection/spicemodels/8345/Zener+Diodes.spice.txt?eid=88) from Diodes Inc with `.ends` statements moved to new lines to appease LTSpice.

All other models used come standard with LTSpiceXVII.

### Installation

To install these libraries, you must copy and paste them into your LTSpice subcircuit folder. Depending on your version of LTSpice, on Windows machines, this could be:
- `%localappdata\LTspice\lib\sub`
- `%userprofile%\Documents\LTSpiceXVII\lib\sub`
- `%programfiles%\LTC\LTSpiceXVII\lib\sub`

or other directories.

# Time Tracking

[Google Sheet](https://docs.google.com/spreadsheets/d/1ABE5ELdahlYolHOQ2TSzXDdkT7J0JBhl6qDKFItKDu4/)

# Ackowledgements

This project is made in collaboration with:
- [Machine Intelligence Laboratory](https://mil.ufl.edu/)
