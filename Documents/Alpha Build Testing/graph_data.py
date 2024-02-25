import csv
import matplotlib.pyplot as plt

with open('rpp-data-2024.02.25_16.11.30.csv', newline='') as csvfile:
    reader = csv.DictReader(csvfile)

    v_applied = []
    v_rail = []
    for col in reader:
        v_applied.append(float(col['Input Voltage (V)']))
        v_rail.append(float(col['3.3V Rail Voltage (V)']))

    fig = plt.figure()
    ax1 = fig.add_subplot(1,1,1)
    fig.supxlabel("Reverse Input Voltage (V)")

    ax1.scatter(v_applied, v_rail)
    ax1.set_ylabel("Rail Voltage (V)")
    ax1.set_title("TGIS Reverse Polarity Protection (3.3V Rail)")

plt.show()
fig.savefig("rpp")

with open('ovp-data-2024.02.25_16.23.02.csv', newline='') as csvfile:
    reader = csv.DictReader(csvfile)

    v_applied = []
    v_rail = []
    v_ovp = []
    for col in reader:
        v_applied.append(float(col['Input Voltage (V)']))
        v_rail.append(float(col['3.3V Rail Voltage (V)']))
        v_ovp.append(float(col['3.3V OVP Circuit Voltage (V)']))

    fig = plt.figure()
    ax1 = fig.add_subplot(2,1,1)
    ax2 = fig.add_subplot(2,1,2)
    fig.supxlabel("Input Voltage (V)")
    fig.suptitle("TGIS Overvoltage Protection (3.3V Rail)")

    ax1.scatter(v_applied, v_rail)
    ax1.set_ylabel("Rail Voltage (V)")

    ax2.scatter(v_applied, v_ovp)
    ax2.set_ylabel("Overvoltage Detection Voltage (V)")

plt.show()
fig.savefig("ovp")