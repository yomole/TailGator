# TailGator

# Introduction

The TailGator Interconnect System (TGIS) project aims to produce a modular connection system and standard for easily connecting, replacing, and upgrading underwater robotics hardware in space-constrained robotics designs. 
Several other standards exist for modular electronics hardware, including the [Adafruit Feather Specification](https://learn.adafruit.com/adafruit-feather/feather-specification), [Sparkfun MicroMod Specification](https://www.sparkfun.com/micromod), [Arduino Shield Specificaton](https://learn.sparkfun.com/tutorials/arduino-shields-v2), and [PCI-e Specification](https://pcisig.com/). However, no single standard contains the pin density, physical dimensions, signal integrity, power delivery, and cost effectiveness we are looking for. The TailGator Interconnect System aims to solve this problem for the Machine Intelligence Laboratories' SubjuGator 9 Autonomous Underwater Vehicle platform while making the design open source to help the robotics community.

# Electrical Specifications

The current design of TGIS can supply:
- 3.3V @ 4A (13.3W)
- 5V @ 4A (20W)

Reserved lines may be used to suppliment the 3.3V line. Board designs may also include a 5V to 3.3V LDO regulator to suppliment the 3.3V line.

# Pinout

![image](https://github.com/yomole/TailGator/assets/80288489/c363241c-c987-4117-98e1-8ad4f62a0eaa)

# Time Tracking

[Google Sheet](https://docs.google.com/document/d/1TKIdP1BIqdC8MU6n8R4Sn8ps7-wJigyKhLupdDVdEag/edit?usp=sharing)

# Ackowledgements

This project is made in collaboration with the [Machine Intelligence Laboratory](https://mil.ufl.edu/)
