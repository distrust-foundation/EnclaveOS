#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <getopt.h>
#include <signal.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/reboot.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/sysmacros.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#include <linux/vm_sockets.h>
#include <poll.h>

_Noreturn void die(const char *msg);
#define die_on(CONDITION, ...) \
    do { \
        if (CONDITION) { \
            die(__VA_ARGS__); \
        } \
    } while (0)

#define finit_module(fd, param_values, flags) (int)syscall(__NR_finit_module, fd, param_values, flags)
#define DEFAULT_PATH_ENV "PATH=/sbin:/usr/sbin:/bin:/usr/bin"
#define NSM_PATH "nsm.ko"
#define TIMEOUT 20000
#define VSOCK_PORT 9000
#define VSOCK_CID 3
#define HEART_BEAT 0xB7

const char *const default_envp[] = {
    DEFAULT_PATH_ENV,
    NULL,
};

const char *const default_argv[] = { "sh", NULL };

struct Mount {
    const char *source, *target, *type;
    unsigned long flags;
    const void *data;
};

struct Mkdir {
    const char *path;
    mode_t mode;
};

struct Mknod {
    const char *path;
    mode_t mode;
    int major, minor;
};

struct Symlink {
    const char *linkpath, *target;
};

enum OpType {
    OpMount,
    OpMkdir,
    OpMknod,
    OpSymlink,
};

struct InitOp {
    enum OpType op;
    union {
        struct Mount mount;
        struct Mkdir mkdir;
        struct Mknod mknod;
        struct Symlink symlink;
    };
};

const struct InitOp ops[] = {
    { OpMount, .mount = { "proc", "/proc", "proc", MS_NODEV | MS_NOSUID | MS_NOEXEC } },
    { OpSymlink, .symlink = { "/dev/fd", "/proc/self/fd" } },
    { OpSymlink, .symlink = { "/dev/stdin", "/proc/self/fd/0" } },
    { OpSymlink, .symlink = { "/dev/stdout", "/proc/self/fd/1" } },
    { OpSymlink, .symlink = { "/dev/stderr", "/proc/self/fd/2" } },
    { OpMkdir, .mkdir = { "/dev/shm", 0755 } },
    { OpMkdir, .mkdir = { "/dev/pts", 0755 } },
    { OpMount, .mount = { "devpts", "/dev/pts", "devpts", MS_NOSUID | MS_NOEXEC } },
    { OpMount, .mount = { "sysfs", "/sys", "sysfs", MS_NODEV | MS_NOSUID | MS_NOEXEC } },
};

void warn(const char *msg) {
    int error = errno;
    perror(msg);
    errno = error;
}

void warn2(const char *msg1, const char *msg2) {
    int error = errno;
    fputs(msg1, stderr);
    fputs(": ", stderr);
    errno = error;
    warn(msg2);
}

_Noreturn void dien() {
    exit(errno);
}

_Noreturn void die(const char *msg) {
    warn(msg);
    dien();
}

_Noreturn void die2(const char *msg1, const char *msg2) {
    warn2(msg1, msg2);
    dien();
}

void init_dev() {
    if (mount("dev", "/dev", "devtmpfs", MS_NOSUID | MS_NOEXEC, NULL) < 0) {
        warn2("mount", "/dev");
        if (errno != EBUSY) {
            dien();
        }
    }
}

void init_console() {
    const char *console_path = "/dev/console";
    die_on(freopen(console_path, "r", stdin) == NULL,
           "freopen failed for stdin");
    die_on(freopen(console_path, "w", stdout) == NULL,
           "freopen failed for stdout");
    die_on(freopen(console_path, "w", stderr) == NULL,
           "freopen failed for stderr");
}

void enclave_ready() {
    int socket_fd;
    struct sockaddr_vm sa = {
        .svm_family = AF_VSOCK,
        .svm_cid = VSOCK_CID,
        .svm_port = VSOCK_PORT,
        .svm_reserved1 = 0,
    };

    uint8_t buf[1];
    buf[0] = HEART_BEAT;
    errno = -EINVAL;

    socket_fd = socket(AF_VSOCK, SOCK_STREAM, 0);
    die_on(socket_fd < 0, "socket");
    die_on(connect(socket_fd, (struct sockaddr*) &sa, sizeof(sa)), "connect");
    die_on(write(socket_fd, buf, 1) != 1, "write heartbeat");
    die_on(read(socket_fd, buf, 1) != 1, "read heartbeat");
    die_on(buf[0] != HEART_BEAT, "received wrong heartbeat");
    die_on(close(socket_fd), "close");
}

void init_nsm_driver() {
    int fd;
    int rc;

    fd = open(NSM_PATH, O_RDONLY | O_CLOEXEC);
    if (fd < 0 && errno == ENOENT) {
        return;
    }
    die_on(fd < 0, "failed to open nsm fd");
    rc = finit_module(fd, "", 0);
    die_on(rc < 0, "failed to insert nsm driver");

    die_on(close(fd), "close nsm fd");
    rc = unlink(NSM_PATH);
    if (rc < 0)
        warn("Could not unlink " NSM_PATH);
}

int main() {
    init_dev();
    init_console();
    init_nsm_driver();
    enclave_ready();
    puts("\nHello World with NSM!\n");
    reboot(RB_AUTOBOOT);
}
