// Rusty-SUNDIALS Web Solver Engine — RK4 + Implicit Euler in JS
const Solver = {
  rk4(f, t0, y0, dt, nsteps) {
    const n = y0.length, ts = [t0], ys = [y0.slice()];
    let t = t0, y = y0.slice();
    const k1=new Float64Array(n),k2=new Float64Array(n),k3=new Float64Array(n),k4=new Float64Array(n),tmp=new Float64Array(n);
    for (let s = 0; s < nsteps; s++) {
      f(t, y, k1);
      for (let i=0;i<n;i++) tmp[i]=y[i]+0.5*dt*k1[i]; f(t+0.5*dt,tmp,k2);
      for (let i=0;i<n;i++) tmp[i]=y[i]+0.5*dt*k2[i]; f(t+0.5*dt,tmp,k3);
      for (let i=0;i<n;i++) tmp[i]=y[i]+dt*k3[i]; f(t+dt,tmp,k4);
      for (let i=0;i<n;i++) y[i]+=(dt/6)*(k1[i]+2*k2[i]+2*k3[i]+k4[i]);
      t += dt;
      ts.push(t); ys.push(y.slice());
    }
    return { ts, ys };
  },
  adaptive(f, t0, y0, tEnd, rtol=1e-6, maxSteps=200000) {
    const n=y0.length, ts=[t0], ys=[y0.slice()];
    let t=t0, y=y0.slice(), h=Math.min((tEnd-t0)/100, 0.01);
    const k1=new Float64Array(n),k2=new Float64Array(n),k3=new Float64Array(n),
          k4=new Float64Array(n),k5=new Float64Array(n),k6=new Float64Array(n),
          y4=new Float64Array(n),y5=new Float64Array(n),tmp=new Float64Array(n);
    // Dormand-Prince RK45 coefficients (simplified Cash-Karp)
    for (let s=0; s<maxSteps && t<tEnd; s++) {
      if (t+h > tEnd) h = tEnd-t;
      f(t,y,k1);
      for(let i=0;i<n;i++) tmp[i]=y[i]+h*0.2*k1[i]; f(t+0.2*h,tmp,k2);
      for(let i=0;i<n;i++) tmp[i]=y[i]+h*(3/40*k1[i]+9/40*k2[i]); f(t+0.3*h,tmp,k3);
      for(let i=0;i<n;i++) tmp[i]=y[i]+h*(0.3*k1[i]-0.9*k2[i]+1.2*k3[i]); f(t+0.6*h,tmp,k4);
      for(let i=0;i<n;i++) tmp[i]=y[i]+h*(-11/54*k1[i]+2.5*k2[i]-70/27*k3[i]+35/27*k4[i]); f(t+h,tmp,k5);
      for(let i=0;i<n;i++) tmp[i]=y[i]+h*(1631/55296*k1[i]+175/512*k2[i]+575/13824*k3[i]+44275/110592*k4[i]+253/4096*k5[i]); f(t+7/8*h,tmp,k6);
      for(let i=0;i<n;i++){
        y4[i]=y[i]+h*(37/378*k1[i]+250/621*k3[i]+125/594*k4[i]+512/1771*k6[i]);
        y5[i]=y[i]+h*(2825/27648*k1[i]+18575/48384*k3[i]+13525/55296*k4[i]+277/14336*k5[i]+0.25*k6[i]);
      }
      let errMax=0;
      for(let i=0;i<n;i++){let sc=Math.abs(y[i])+Math.abs(h*k1[i])+1e-30; errMax=Math.max(errMax,Math.abs(y4[i]-y5[i])/sc);}
      errMax/=rtol;
      if(errMax<=1){t+=h;for(let i=0;i<n;i++)y[i]=y4[i];ts.push(t);ys.push(y.slice());h*=Math.min(5,0.9*Math.pow(errMax,-0.2));}
      else{h*=Math.max(0.1,0.9*Math.pow(errMax,-0.25));if(h<1e-15)break;}
    }
    return {ts,ys,steps:ts.length-1};
  }
};
