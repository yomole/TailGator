//*****************************************************************************
//
// blinky.c - Simple example to blink the on-board LED.
//
// Copyright (c) 2013-2020 Texas Instruments Incorporated.  All rights reserved.
// Software License Agreement
// 
// Texas Instruments (TI) is supplying this software for use solely and
// exclusively on TI's microcontroller products. The software is owned by
// TI and/or its suppliers, and is protected under applicable copyright
// laws. You may not combine this software with "viral" open-source
// software in order to form a larger program.
// 
// THIS SOFTWARE IS PROVIDED "AS IS" AND WITH ALL FAULTS.
// NO WARRANTIES, WHETHER EXPRESS, IMPLIED OR STATUTORY, INCLUDING, BUT
// NOT LIMITED TO, IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE APPLY TO THIS SOFTWARE. TI SHALL NOT, UNDER ANY
// CIRCUMSTANCES, BE LIABLE FOR SPECIAL, INCIDENTAL, OR CONSEQUENTIAL
// DAMAGES, FOR ANY REASON WHATSOEVER.
// 
// This is part of revision 2.2.0.295 of the EK-TM4C1294XL Firmware Package.
//
//*****************************************************************************

#include <stdint.h>
#include <stdbool.h>
#include "inc/hw_memmap.h"
#include "driverlib/debug.h"
#include "driverlib/gpio.h"
#include "driverlib/sysctl.h"

#include "inc/hw_i2c.h"
#include "inc/hw_types.h"
#include "driverlib/i2c.h"
#include "driverlib/pin_map.h"
#include "driverlib/uart.h"
#include "uartstdio.h"

#define BMI160_ADDRESS 0x69

uint32_t ui32SysClock;


// Set up UART0 for console connectivity
void initConsole(void) {
    // Enable GPIO port A for UART0 pins
    SysCtlPeripheralEnable(SYSCTL_PERIPH_GPIOA);

    // Configure A0 and A1 to be the TX and RX for UART0
    GPIOPinConfigure(GPIO_PA0_U0RX);
    GPIOPinConfigure(GPIO_PA1_U0TX);

    // Enable UART0
    SysCtlPeripheralEnable(SYSCTL_PERIPH_UART0);

    // Use the internal 16MHz oscillator as the UART clock source
    UARTClockSourceSet(UART0_BASE, UART_CLOCK_PIOSC);

    // Select UART function for A0 and A1
    GPIOPinTypeUART(GPIO_PORTA_BASE, GPIO_PIN_0 | GPIO_PIN_1);

    // Configure UART0 for console IO
    UARTStdioConfig(0, 115200, 16000000);
}

// Read in a single byte of data from an I2C peripheral
uint8_t I2CReadSingle(uint8_t periphAddr) {
    I2CMasterSlaveAddrSet(I2C0_BASE, periphAddr, true);

    I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_SINGLE_RECEIVE);

    //SysCtlDelay(250);
    while(!I2CMasterBusy(I2C0_BASE));
    while(I2CMasterBusy(I2C0_BASE));

    return I2CMasterDataGet(I2C0_BASE);
}

// Write a single byte of data to an I2C peripheral
void I2CWriteSingle(uint8_t periphAddr,  uint8_t data) {
    I2CMasterSlaveAddrSet(I2C0_BASE, periphAddr, false);

    I2CMasterDataPut(I2C0_BASE, data);

    I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_SINGLE_SEND);

    //SysCtlDelay(250);
    while(!I2CMasterBusy(I2C0_BASE));
    while(I2CMasterBusy(I2C0_BASE));
}

// Read a single register on the BMI160 IMU
uint8_t BMI160ReadSingleRegister(uint8_t registerAddr) {
    I2CWriteSingle(BMI160_ADDRESS, registerAddr);
    return I2CReadSingle(BMI160_ADDRESS);
}


void I2CReadMultiple(uint8_t periphAddr, uint8_t size, uint8_t *result) {
    I2CMasterSlaveAddrSet(I2C0_BASE, periphAddr, true);

    I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_BURST_RECEIVE_START);
    //SysCtlDelay(250);
    while(!I2CMasterBusy(I2C0_BASE));
    while(I2CMasterBusy(I2C0_BASE));
    result[0] = I2CMasterDataGet(I2C0_BASE);

    for(uint8_t i = 1; i < size-1; i++) {
        I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_BURST_RECEIVE_CONT);
        //SysCtlDelay(250);
        while(!I2CMasterBusy(I2C0_BASE));
        while(I2CMasterBusy(I2C0_BASE));
        result[i] = I2CMasterDataGet(I2C0_BASE);
    }

    I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_BURST_RECEIVE_FINISH);
    //SysCtlDelay(250);
    while(!I2CMasterBusy(I2C0_BASE));
    while(I2CMasterBusy(I2C0_BASE));
    result[size-1] = I2CMasterDataGet(I2C0_BASE);
}


void I2CWriteMultiple(uint8_t periphAddr, uint8_t size, uint8_t *data) {
    I2CMasterSlaveAddrSet(I2C0_BASE, periphAddr, false);

    I2CMasterDataPut(I2C0_BASE, data[0]);
    I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_BURST_SEND_START);
    //SysCtlDelay(250);
    while(!I2CMasterBusy(I2C0_BASE));
    while(I2CMasterBusy(I2C0_BASE));

    for(uint8_t i = 1; i < size-1; i++) {
        I2CMasterDataPut(I2C0_BASE, data[i]);
        I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_BURST_SEND_CONT);
        //SysCtlDelay(250);
        while(!I2CMasterBusy(I2C0_BASE));
        while(I2CMasterBusy(I2C0_BASE));
    }

    I2CMasterDataPut(I2C0_BASE, data[size-1]);
    I2CMasterControl(I2C0_BASE, I2C_MASTER_CMD_BURST_SEND_FINISH);
    //SysCtlDelay(250);
    while(!I2CMasterBusy(I2C0_BASE));
    while(I2CMasterBusy(I2C0_BASE));
}

// Read multiple registers on the BMI160 IMU
void BMI160ReadMultipleRegisters(uint8_t startRegisterAddr, uint8_t size, uint8_t *result) {
    I2CWriteSingle(BMI160_ADDRESS, startRegisterAddr);
    I2CReadMultiple(BMI160_ADDRESS, size, result);
}

uint8_t BMI160GetDataStatus() {
    return BMI160ReadSingleRegister(0x1b);
}

void BMI160Init() {
    uint8_t data[2] = {0x7e, 0x15};
    I2CWriteMultiple(BMI160_ADDRESS, sizeof(data), &data[0]);

//    data[1] = 0x11;
//    I2CWriteMultiple(BMI160_ADDRESS, sizeof(data), &data[0]);
}



int main(void)
{
    // Clock definition
//    ui32SysClock = SysCtlClockFreqSet((SYSCTL_XTAL_25MHZ |
//                                           SYSCTL_OSC_MAIN |
//                                           SYSCTL_USE_OSC), 25000000);

    // ---------------- Begin I2C Config ---------------------------------- //
    // Enable Port B (used by I2C0: B2/B3)
    SysCtlPeripheralEnable(SYSCTL_PERIPH_GPIOB);

    // Enable I2C0
    SysCtlPeripheralEnable(SYSCTL_PERIPH_I2C0);

    // Set B2/3 to I2C functionality
    GPIOPinConfigure(GPIO_PB2_I2C0SCL);
    GPIOPinConfigure(GPIO_PB3_I2C0SDA);

    // Set up pins
    GPIOPinTypeI2CSCL(GPIO_PORTB_BASE, GPIO_PIN_2);
    GPIOPinTypeI2C(GPIO_PORTB_BASE, GPIO_PIN_3);

    // Init I2C clock
    I2CMasterInitExpClk(I2C0_BASE, SysCtlClockGet(), false);
    // ---------------- End I2C Config ------------------------------------ //

    // Hello-world to console
    initConsole();
    UARTprintf("I2C Hello World!\r\n");

    // Read status address register (0x1B)
    uint8_t id = BMI160ReadSingleRegister(0x00);
    UARTprintf("ID: %x\r\n", id);

    // Init BMI160
    BMI160Init();

    // Wait for data status to appear from accelerometer
    uint8_t status = BMI160GetDataStatus();
    UARTprintf("Status: %x\r\n", status);
    while(!(status & 0x80)) {
        UARTprintf("Status: %x\r\n", status);
        status = BMI160GetDataStatus();
    }


    // Get x accelerometer data
    uint8_t result[2] = {0, 0};

    // 0x04 = data start addr for all components
    // 14 = x accelerometer
    BMI160ReadMultipleRegisters(0x04+14, sizeof(result), result);

    int16_t x_accel = (int16_t)(result[1]) << 8 | result[0];
    UARTprintf("x accel: %d\r\n", x_accel);





}
