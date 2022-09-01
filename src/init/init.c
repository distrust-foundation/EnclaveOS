#include <stdio.h>
#include <unistd.h>

int main(int argc, char **argv) {
    (void) argc;
    (void) argv;
    puts("Appliance Linux Sample Init");
    sleep(0xFFFFFFFF);
}
