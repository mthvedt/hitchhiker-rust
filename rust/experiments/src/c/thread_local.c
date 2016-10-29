#include <sys/time.h>
#include <inttypes.h>

typedef struct ThreadLocal {
    int64_t i;
} ThreadLocal;

__thread ThreadLocal* for_rust;

void create_thread_local() {
    for_rust = (ThreadLocal*) malloc(sizeof(ThreadLocal));
}

ThreadLocal* thread_local() {
    return for_rust;
}

__thread ThreadLocal* for_c;
void c_thread_local() {
    for_c = (ThreadLocal*) malloc(sizeof(ThreadLocal));

    struct timeval start, end;

    gettimeofday(&start, NULL);
    int64_t i = 1;
    for (; i < 5000000; i++)
        for_c->i = for_c->i + i;
    gettimeofday(&end, NULL);

    int duration = (int) ((double) (end.tv_usec - start.tv_usec) / 1000 + (double) (end.tv_sec - start.tv_sec) * 1000);

    printf("C: %" PRIi64 "\n", for_c->i);
    printf("C: %d msec\n", duration);
}
