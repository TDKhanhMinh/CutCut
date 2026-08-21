import type { AuthChangeEvent, AuthResponse, Session } from "@supabase/supabase-js";
import { supabase } from "@/lib/supabase";

/** Typed boundary for cloud auth; components and stores do not own Supabase calls. */
export const authService = {
  getSession: (): Promise<{ data: { session: Session | null }; error: Error | null }> =>
    supabase.auth.getSession(),
  onAuthStateChange: (
    callback: (event: AuthChangeEvent, session: Session | null) => void,
  ) => supabase.auth.onAuthStateChange(callback),
  signIn: (email: string, password: string): Promise<AuthResponse> =>
    supabase.auth.signInWithPassword({ email: email.trim(), password }),
  signUp: (email: string, password: string): Promise<AuthResponse> =>
    supabase.auth.signUp({ email: email.trim(), password }),
  signOut: () => supabase.auth.signOut({ scope: "local" }),
  invokeFunction: <T>(functionName: string, body: Record<string, unknown>) =>
    supabase.functions.invoke<T>(functionName, { body }),
};
