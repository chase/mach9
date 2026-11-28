/* Print the native executable path for the signed-loader smoke test. */
#include <libproc.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv)
{
    char path[PROC_PIDPATHINFO_MAXSIZE];

    if (argc != 2) return 1;
    if (proc_pidpath(atoi(argv[1]), path, sizeof(path)) <= 0)
    {
        perror("proc_pidpath");
        return 1;
    }
    puts(path);
    return 0;
}
