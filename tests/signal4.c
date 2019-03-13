#include <sys/types.h>
#include <sys/syscall.h>
#include <unistd.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <assert.h>
#include <errno.h>
#include <pthread.h>

static volatile int quit = 0;

static void handler(int sig, siginfo_t *info, void *ucontext)
{
  quit = 1;
}

void* thread_main(void* param)
{
  struct timespec ts = {0, 100000000};
  nanosleep(&ts, NULL);
  _exit(1);
}

int main(int argc, char* argv[])
{
  int ret;
  sigset_t sigset;
  struct sigaction old, new;

  pthread_t tid;
  
  pthread_create(&tid, NULL, thread_main, (void*)1UL);
  
  memset(&old, 0, sizeof(old));
  memset(&new, 0, sizeof(new));

  sigemptyset(&sigset);
  new.sa_sigaction = handler;
  new.sa_mask = sigset;
  new.sa_flags = SA_RESTART | SA_SIGINFO;
  
  ret = sigaction(SIGALRM, &new, &old);
  if (ret < 0) {
    perror("rt_sigaction");
    exit(1);
  }

  ret = sigaction(SIGALRM, NULL, &old);
  if (ret < 0) {
    perror("rt_sigaction");
    exit(1);
  }

  assert((unsigned long)old.sa_sigaction == (unsigned long)handler);

  alarm(1);

  while(!quit);

  return 0;
}
