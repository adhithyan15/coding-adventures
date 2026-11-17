/*
 * Sensitivity search for the conformance-vector gates (PHY00a 6.4).
 *
 * Question: does a *variant* of TRIG-REF-1 (the last bit of one constant flipped, or one
 * of the 24 sites computed as a fused multiply-add) return different bits from the
 * reference on any of N pseudo-random inputs?  If yes, the hits are candidates for a
 * directed vector (WITNESSES in generate_trig_reference_fixture.py, which re-verifies
 * them with the soft-float model).  If no hit shows in a large N, the item goes on the
 * masked list.
 *
 *   gcc -O2 -ffp-contract=off -o search search.c -lm
 *   ./search mut  C3 0 300000000 1      # KIND NAME MODE N SEED
 *   ./search fuse 21 0 300000000 1      # a fusion site is its SITES index (0..23)
 *
 * NAME for `mut` is S1..S9, C2..C9 or PIO2_5.  Fusion sites are numbered as in SITES:
 * 0 red.t, 1-5 red.1..5, 6-13 sin.h8..h1, 14 sin.fin, 15-21 cos.h8..h2, 22 cos.half,
 * 23 cos.fin.  MODE picks the input domain:
 *   0  uniform in [1/2, pi/4): k = 0 and |r| is largest, so polynomial rounding matters most
 *   1  random exponent in [-3, 28], random mantissa: the whole accurate domain
 *   2  random exponent in [0, 28]: large arguments, which exercise the reduction steps
 * fma() is the C library's correctly rounded fused multiply-add.  This program is a
 * search aid, not part of the fixture: its output is never read by the generator.
 */
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static double F(uint64_t b){double d;memcpy(&d,&b,8);return d;}
static uint64_t B(double d){uint64_t b;memcpy(&b,&d,8);return b;}
typedef struct { double INV,P[6],MAGIC,S[10],C[10]; unsigned fuse; } M;
static const uint64_t sb[9]={0xBFC5555555555555ull,0x3F81111111111111ull,0xBF2A01A01A01A01Aull,0x3EC71DE3A556C734ull,0xBE5AE64567F544E4ull,0x3DE6124613A86D09ull,0xBD6AE7F3E733B81Full,0x3CE952C77030AD4Aull,0xBC62F49B46814157ull};
static const uint64_t cb[8]={0x3FA5555555555555ull,0xBF56C16C16C16C17ull,0x3EFA01A01A01A01Aull,0xBE927E4FB7789F5Cull,0x3E21EED8EFF8D898ull,0xBDA93974A8C07C9Dull,0x3D2AE7F3E733B81Full,0xBCA6827863B97D97ull};
static const uint64_t pb[5]={0x3FF921FB40000000ull,0x3E74442D00000000ull,0x3CF8469880000000ull,0x3B78CC5160000000ull,0x39F01B839A25204Aull};
static void init(M*m,int mutS,int mutC,int mutP,unsigned fuse){
  m->INV=F(0x3FE45F306DC9C883ull);m->MAGIC=F(0x4338000000000000ull);
  for(int i=0;i<5;i++)m->P[i+1]=F(pb[i]^(mutP==i+1));
  for(int n=1;n<=9;n++)m->S[n]=F(sb[n-1]^(mutS==n));
  for(int n=2;n<=9;n++)m->C[n]=F(cb[n-2]^(mutC==n));
  m->fuse=fuse;}
/* site bit numbering = SITES order: 0 red.t,1-5 red.i,6..13 sin.h8..h1,14 sin.fin,15..21 cos.h8..h2,22 cos.half,23 cos.fin */
#define FU(m,i) ((m)->fuse>>(i)&1)
static double mad(const M*m,int site,double a,double b,double c){return FU(m,site)?fma(a,b,c):a*b+c;}
static double sin_k(const M*m,double r){double z=r*r,p=m->S[9];
  for(int n=8;n>=1;n--)p=mad(m,6+(8-n),p,z,m->S[n]);
  double w=r*z; if(FU(m,14))return fma(w,p,r); w=w*p; return r+w;}
static double cos_k(const M*m,double r){double z=r*r,q=m->C[9];
  for(int n=8;n>=2;n--)q=mad(m,15+(8-n),q,z,m->C[n]);
  double u; if(FU(m,22))u=fma(-0.5,z,1.0); else u=1.0-0.5*z;
  double v=z*z; if(FU(m,23))return fma(v,q,u); v=v*q; return u+v;}
static void reduce(const M*m,double x,double*k,double*r){
  double u; if(FU(m,0))u=fma(x,m->INV,m->MAGIC); else u=x*m->INV+m->MAGIC;
  double kk=u-m->MAGIC,rr=x;
  for(int i=1;i<=5;i++){ if(FU(m,i))rr=fma(-kk,m->P[i],rr); else {double t=kk*m->P[i]; rr=rr-t;} }
  *k=kk;*r=rr;}
static double sn(const M*m,double x){double k,r;reduce(m,x,&k,&r);int q=(int)k&3;
  return q==0?sin_k(m,r):q==1?cos_k(m,r):q==2?-sin_k(m,r):-cos_k(m,r);}
static double cs(const M*m,double x){double k,r;reduce(m,x,&k,&r);int q=(int)k&3;
  return q==0?cos_k(m,r):q==1?-sin_k(m,r):q==2?-cos_k(m,r):sin_k(m,r);}
static uint64_t s[4];
static uint64_t rotl(uint64_t x,int k){return(x<<k)|(x>>(64-k));}
static uint64_t nx(void){uint64_t r=rotl(s[1]*5,7)*9,t=s[1]<<17;s[2]^=s[0];s[3]^=s[1];s[1]^=s[2];s[0]^=s[3];s[2]^=t;s[3]=rotl(s[3],45);return r;}
int main(int c,char**v){
  const char*kind=v[1],*name=v[2];int mode=atoi(v[3]);long N=atol(v[4]);uint64_t st=strtoull(v[5],0,10);
  for(int i=0;i<4;i++){st+=0x9E3779B97F4A7C15ull;uint64_t z=st;z=(z^(z>>30))*0xBF58476D1CE4E5B9ull;z=(z^(z>>27))*0x94D049BB133111EBull;s[i]=z^(z>>31);}
  M ref,var; int mS=0,mC=0,mP=0;unsigned fu=0;
  if(!strcmp(kind,"mut")){int n=atoi(name+1);if(name[0]=='S')mS=n;else if(name[0]=='C')mC=n;else mP=5;}
  else fu=1u<<atoi(name);
  init(&ref,0,0,0,0);init(&var,mS,mC,mP,fu);
  long hits=0;int shown=0;
  for(long i=0;i<N;i++){
    double x; uint64_t u=nx();
    if(mode==0){ x=F(0x3FE0000000000000ull+(u>>12)%(0x3FE921FB54442D18ull-0x3FE0000000000000ull)); }
    else if(mode==1){ int e=-3+(int)((u>>56)%32); x=F(((uint64_t)(1023+e)<<52)|(u&0xFFFFFFFFFFFFFull)); }
    else { int e=0+(int)((u>>56)%29); x=F(((uint64_t)(1023+e)<<52)|(u&0xFFFFFFFFFFFFFull)); }
    for(int op=0;op<2;op++){
      double a=op?cs(&ref,x):sn(&ref,x),b=op?cs(&var,x):sn(&var,x);
      if(B(a)!=B(b)){hits++; if(shown<8){printf("%s %s mode %d hit %s %016llx\n",kind,name,mode,op?"cos":"sin",(unsigned long long)B(x));shown++;} break;}
    }
  }
  printf("%s %s mode %d: %ld / %ld\n",kind,name,mode,hits,N);return 0;}
