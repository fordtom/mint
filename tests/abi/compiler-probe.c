#include "mint_abi.h"
#include "mint_abi.h"
#include "mint_pack.h"
#include "mint_pack.h"

#define ASSERT_ABI(type, bits, alignment)                                      \
  _Static_assert(sizeof(type) * CHAR_BIT == bits, #type " storage");           \
  _Static_assert(_Alignof(type) * CHAR_BIT == alignment, #type " alignment")

#if defined(MINT_TI_C28X)
#if !defined(__TMS320C28XX__) || !defined(__TI_EABI__)
#error "TI C28x EABI compiler required"
#endif
_Static_assert(CHAR_BIT == 16, "C28x must use 16-bit addressable units");
_Static_assert(__little_endian__ == 1, "C28x must be little-endian");
ASSERT_ABI(uint16_t, 16, 16);
ASSERT_ABI(int16_t, 16, 16);
ASSERT_ABI(uint32_t, 32, 32);
ASSERT_ABI(int32_t, 32, 32);
ASSERT_ABI(uint64_t, 64, 32);
ASSERT_ABI(int64_t, 64, 32);
ASSERT_ABI(float, 32, 32);
ASSERT_ABI(double, 64, 32);
#elif defined(MINT_TRICORE)
_Static_assert(CHAR_BIT == 8, "byte-addressed profile required");
ASSERT_ABI(uint8_t, 8, 8);
ASSERT_ABI(int8_t, 8, 8);
ASSERT_ABI(uint16_t, 16, 16);
ASSERT_ABI(int16_t, 16, 16);
ASSERT_ABI(uint32_t, 32, 32);
ASSERT_ABI(int32_t, 32, 32);
ASSERT_ABI(uint64_t, 64, 32);
ASSERT_ABI(int64_t, 64, 32);
ASSERT_ABI(float, 32, 32);
ASSERT_ABI(double, 64, 32);
#if defined(__BYTE_ORDER__) && defined(__ORDER_LITTLE_ENDIAN__)
_Static_assert(__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__,
               "little-endian compiler required");
#endif
#else
#if defined(MINT_ARM) && !defined(__arm__)
#error "Arm compiler required"
#elif defined(MINT_RISCV) && (!defined(__riscv) || __riscv_xlen != 32)
#error "32-bit RISC-V compiler required"
#endif
_Static_assert(CHAR_BIT == 8, "byte-addressed profile required");
ASSERT_ABI(uint8_t, 8, 8);
ASSERT_ABI(int8_t, 8, 8);
ASSERT_ABI(uint16_t, 16, 16);
ASSERT_ABI(int16_t, 16, 16);
ASSERT_ABI(uint32_t, 32, 32);
ASSERT_ABI(int32_t, 32, 32);
ASSERT_ABI(uint64_t, 64, 64);
ASSERT_ABI(int64_t, 64, 64);
ASSERT_ABI(float, 32, 32);
ASSERT_ABI(double, 64, 64);
#if defined(MINT_EXPECT_BIG_ENDIAN)
_Static_assert(__BYTE_ORDER__ == __ORDER_BIG_ENDIAN__,
               "big-endian compiler required");
#else
_Static_assert(__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__,
               "little-endian compiler required");
#endif
#endif
_Static_assert(sizeof(((config_t *)0)->coefficients) == 4 * sizeof(float),
               "array stride");
_Static_assert(sizeof(((config_t *)0)->matrix) == 4 * sizeof(int16_t),
               "matrix stride");
_Static_assert(sizeof(((pack_t *)0)->coefficients) == 4 * sizeof(float),
               "pack array stride");
_Static_assert(sizeof(((pack_t *)0)->matrix) == 4 * sizeof(int16_t),
               "pack matrix stride");
int mint_abi_probe(config_t *config, data_t *data, pack_t *pack) {
  return (int)(config->device.id + config->coefficients[1] + data->counter +
               pack->word + pack->nested.wide + pack->nested.tail);
}
