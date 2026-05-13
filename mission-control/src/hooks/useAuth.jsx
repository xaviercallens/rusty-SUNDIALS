import { createContext, useContext, useState, useEffect, useCallback } from 'react';

const GOOGLE_CLIENT_ID = '1003063861791-gdun1ecne4v4j67eo3k4i88498nkdefs.apps.googleusercontent.com';
const ADMIN_EMAIL = 'callensxavier@gmail.com';

const AuthContext = createContext({
  role: 'guest', email: '', token: null, loading: true,
  signIn: () => {}, signOut: () => {},
});

export function AuthProvider({ children }) {
  const [auth, setAuth] = useState({
    role: 'guest', email: '', token: null, loading: true,
  });

  const handleCredentialResponse = useCallback((response) => {
    const token = response.credential;
    // Decode JWT payload (base64url)
    const payload = JSON.parse(atob(token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')));
    const email = payload.email || '';
    const role = email === ADMIN_EMAIL ? 'admin' : 'guest';
    setAuth({ role, email, token, loading: false });
    localStorage.setItem('mc_token', token);
  }, []);

  const signOut = useCallback(() => {
    setAuth({ role: 'guest', email: '', token: null, loading: false });
    localStorage.removeItem('mc_token');
    if (window.google?.accounts?.id) {
      window.google.accounts.id.disableAutoSelect();
    }
  }, []);

  const signIn = useCallback(() => {
    if (window.google?.accounts?.id) {
      window.google.accounts.id.prompt();
    }
  }, []);

  useEffect(() => {
    // Try to restore token from localStorage
    const saved = localStorage.getItem('mc_token');
    if (saved) {
      try {
        const payload = JSON.parse(atob(saved.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')));
        // Check if token is expired
        if (payload.exp * 1000 > Date.now()) {
          const email = payload.email || '';
          setAuth({
            role: email === ADMIN_EMAIL ? 'admin' : 'guest',
            email, token: saved, loading: false,
          });
          return;
        }
      } catch { /* ignore */ }
      localStorage.removeItem('mc_token');
    }

    // Initialize Google Sign-In
    const initGSI = () => {
      if (window.google?.accounts?.id) {
        window.google.accounts.id.initialize({
          client_id: GOOGLE_CLIENT_ID,
          callback: handleCredentialResponse,
          auto_select: true,
        });
        // Render a hidden prompt for auto-select
        window.google.accounts.id.prompt();
        setAuth(prev => ({ ...prev, loading: false }));
      } else {
        setTimeout(initGSI, 500);
      }
    };
    initGSI();
  }, [handleCredentialResponse]);

  return (
    <AuthContext.Provider value={{ ...auth, signIn, signOut }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}

export default AuthContext;
