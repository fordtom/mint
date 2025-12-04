#ifndef EXAMPLES_BLOCKS_H
#define EXAMPLES_BLOCKS_H

#include <stdint.h>

/*
 * Example C structs corresponding to the example blocks in
 * examples/block.{toml,yaml,json}.
 *
 * Field order matches layout emission order. Arrays reflect the declared sizes.
 * This mapping assumes standard C alignment; the builder inserts alignment
 * padding before each field based on its scalar size, which typically matches
 * how most compilers lay out structs. Verify on your target if strict binary
 * compatibility is required.
 */

/* Array-of-structs examples */
typedef struct {
  float A;
  float B;
} AStruct; /* corresponds to f32[2] per row */

typedef struct {
  float x;
  float y;
  float z;
} Point3f; /* corresponds to f32[3] per row */

/* Deeply nested example types */
typedef struct {
  uint16_t scalar16;
  int16_t array1d[4];
} DeepLevel3;

typedef struct {
  DeepLevel3 level3;
} DeepLevel2;

typedef struct {
  DeepLevel2 level2;
} DeepLevel1;

typedef struct {
  DeepLevel1 level1;
} NestedComplex;

typedef struct {
  /* some.struct.* */
  uint32_t some_struct_value;
  uint32_t some_struct_value2;
  uint8_t some_struct_value3[10];

  /* device.info.* */
  uint8_t device_info_name[16];
  uint32_t device_info_serial;
  uint16_t device_info_version_major;
  uint16_t device_info_version_minor;
  uint16_t device_info_version_patch;

  /* wifi.* and net.* */
  uint8_t wifi_ssid[32];
  uint8_t wifi_key[64];
  uint8_t net_ip[4];

  /* calibration.* */
  float calibration_coefficients[8];
  int16_t calibration_matrix[3][3];

  /* message and magic */
  uint8_t message[16];
  uint32_t magic;

  /* deeper nesting (inline scalar and 1D array) */
  NestedComplex
      nested_complex; /* maps to nested.complex.level1.level2.level3.* */

  /* arrays-of-structs as 2D arrays */
  AStruct structs_astruct_array[10]; /* structs.astruct_array size=[10,2] */
} block_t;

typedef struct {
  uint16_t another_struct_value[10][2];
  uint16_t another_struct_arr[2];
  uint8_t another_struct_description[32];
} block2_t;

typedef struct {
  uint64_t counters_boot_count;
  int16_t limits_temperature_min;
  int16_t limits_temperature_max;
  float thresholds_voltage[4];
  uint8_t dlegal_notice[128];
} block3_t;

#endif /* EXAMPLES_BLOCKS_H */