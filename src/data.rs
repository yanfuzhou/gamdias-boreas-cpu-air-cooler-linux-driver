// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024 BOREAS Linux Project Contributors

pub const PRODUCTS: [u16; 2] = [
    0xB554, 0xB53A
];

pub const CPU_TEM0: [&str; 3] = [
    "coretemp", "k10temp", "zenpower"
];

pub const CPU_TEM1: [&str; 2] = [
    "cpu_thermal", "acpitz"
];

pub const CPU_FAN0: [&str; 45] = [
    "nct6687", "nct6798", "nct6775", "it8603", "it8606", 
    "it8607",  "it8613",  "it8620",  "it8622", "it8623", 
    "it8625",  "it8628",  "it8528",  "it8655", "it8665", 
    "it8686",  "it8688",  "it8689",  "it8696", "it8698", 
    "it8705",  "it8712",  "it8716",  "it8718", "it8720", 
    "it8721",  "it8726",  "it8728",  "it8732", "it8736", 
    "it8738",  "it8758",  "it8771",  "it8772", "it8781", 
    "it8782",  "it8783",  "it8786",  "it8790", "it8792", 
    "it87952", "Sis950",  "nct",     "asus",   "dell"
];