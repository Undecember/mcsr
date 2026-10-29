#include "keyd.h"

static struct key_event recorded[32];
static size_t used;
static void emit(uint8_t code, uint8_t pressed)
{
    assert(used < ARRAY_SIZE(recorded));
    recorded[used++] = (struct key_event){.code = code, .pressed = pressed};
}
static void layer(const struct keyboard *kbd, const struct layer *state, uint8_t active) {}
static void expect(size_t index, uint8_t code, uint8_t pressed)
{
    assert(index < used);
    assert(recorded[index].code == code && recorded[index].pressed == pressed);
}
int main(int argc, char **argv)
{
    struct config config;
    struct output output = {.send_key = emit, .on_layer_change = layer};
    assert(argc == 2 && config_parse(&config, argv[1]) == 0);
    struct keyboard *kbd = new_keyboard(&config, &output);
    struct key_event held[] = {{KEYD_TAB, 1, 0}, {KEYD_B, 1, 1}};
    kbd_process_events(kbd, held, ARRAY_SIZE(held));
    assert(used == 2);
    expect(0, KEYD_F3, 1); expect(1, KEYD_B, 1);
    assert(kbd_eval(kbd, "reset") == 0);
    assert(used == 4);
    for (size_t i = 2; i < 4; i++) assert(recorded[i].pressed == 0);
    struct key_event after[] = {{KEYD_TAB, 0, 2}, {KEYD_B, 0, 3}, {KEYD_B, 1, 4}, {KEYD_B, 0, 5}};
    kbd_process_events(kbd, after, ARRAY_SIZE(after));
    assert(used == 6);
    expect(4, KEYD_F13, 1); expect(5, KEYD_F13, 0);
    free(kbd);
    puts("reset held-key cleanup PASSED");
    return 0;
}
