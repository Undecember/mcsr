#include "daemon.c"

/* The bind path does not touch devices; fail if a different path does. */
struct device device_table[MAX_DEVICES];
size_t device_table_sz;
void vkbd_send_key(const struct vkbd *v, uint8_t code, int state) { abort(); }
void vkbd_mouse_scroll(const struct vkbd *v, int x, int y) { abort(); }
void device_set_led(const struct device *dev, int led, int state) { abort(); }
int device_grab(struct device *dev) { abort(); }
int device_ungrab(struct device *dev) { abort(); }

static void emit(uint8_t code, uint8_t pressed) {}
static void layer(const struct keyboard *kbd, const struct layer *state, uint8_t active) {}

static void expect_bind_bytes(const char *expression, size_t length, int expected)
{
    int sockets[2];
    struct ipc_message request = {.type = IPC_BIND};
    struct ipc_message response;
    assert(socketpair(AF_UNIX, SOCK_STREAM, 0, sockets) == 0);
    request.sz = length;
    memcpy(request.data, expression, request.sz);
    xwrite(sockets[0], &request, sizeof request);
    handle_client(sockets[1]);
    xread(sockets[0], &response, sizeof response);
    close(sockets[0]);
    if (response.type != expected) {
        fprintf(stderr, "bind '%s': expected %d, got %d\n", expression, expected, response.type);
        exit(1);
    }
}

static void expect_bind(const char *expression, int expected)
{
    expect_bind_bytes(expression, strlen(expression), expected);
}

int main(int argc, char **argv)
{
    struct config_ent entry = {0};
    struct output output = {.send_key = emit, .on_layer_change = layer};
    assert(argc == 2);
    expect_bind("reset", IPC_SUCCESS);
    expect_bind("a = b", IPC_FAIL);
    expect_bind("invalid", IPC_FAIL);
    expect_bind("reset ", IPC_FAIL);
    expect_bind("resetx", IPC_FAIL);
    expect_bind_bytes("reset\0extra", 11, IPC_FAIL);
    assert(config_parse(&entry.config, argv[1]) == 0);
    entry.kbd = new_keyboard(&entry.config, &output);
    configs = &entry;
    expect_bind("reset", IPC_SUCCESS);
    expect_bind("invalid", IPC_FAIL);
    configs = NULL;
    free(entry.kbd);
    puts("IPC bind reset PASSED");
    return 0;
}
