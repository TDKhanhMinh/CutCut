import { create } from "zustand";
import type { AuthChangeEvent, Session, User } from "@supabase/supabase-js";
import { authService } from "@/services/auth";
import { useEntitlementStore } from "@/stores/useEntitlementStore";

export type AuthStatus = "initializing" | "signed_in" | "signed_out" | "session_expired" | "offline";

interface AuthState {
  user: User | null;
  session: Session | null;
  status: AuthStatus;
  error: string | null;
  isInitialized: boolean;
  initialize: () => Promise<void>;
  signIn: (email: string, password: string) => Promise<void>;
  signUp: (email: string, password: string) => Promise<void>;
  signOut: () => Promise<void>;
}

let authSubscription: { unsubscribe: () => void } | null = null;
let initializePromise: Promise<void> | null = null;
let explicitSignOut = false;

const safeErrorMessage = (error: unknown) =>
  error instanceof Error && error.message.trim() ? error.message : "Authentication unavailable";

const updateSession = (
  set: (state: Partial<AuthState>) => void,
  session: Session | null,
  event?: AuthChangeEvent,
) => {
  const user = session?.user ?? null;
  const status: AuthStatus = session
    ? "signed_in"
    : event === "SIGNED_OUT" && !explicitSignOut
      ? "session_expired"
      : "signed_out";

  set({ session, user, status, error: null, isInitialized: true });
  if (user) {
    void useEntitlementStore.getState().fetchEntitlements(user.id);
  } else {
    useEntitlementStore.getState().clearEntitlements();
  }
};

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  session: null,
  status: "initializing",
  error: null,
  isInitialized: false,

  initialize: async () => {
    if (initializePromise) return initializePromise;
    const current = useAuthStore.getState();
    if (current.isInitialized && current.status !== "offline") return;

    initializePromise = (async () => {
      set({ status: "initializing", error: null });
      try {
        const {
          data: { session },
          error,
        } = await authService.getSession();
        if (error) throw error;

        updateSession(set, session, "INITIAL_SESSION");

        if (!authSubscription) {
          const { data } = authService.onAuthStateChange((event, nextSession) => {
            updateSession(set, nextSession, event);
          });
          authSubscription = data.subscription;
        }
      } catch (error) {
        set({
          session: null,
          user: null,
          status: "offline",
          error: safeErrorMessage(error),
          isInitialized: true,
        });
        useEntitlementStore.getState().clearEntitlements();
      } finally {
        initializePromise = null;
      }
    })();

    return initializePromise;
  },

  signIn: async (email, password) => {
    const { error } = await authService.signIn(email, password);
    if (error) throw error;
  },

  signUp: async (email, password) => {
    const { error } = await authService.signUp(email, password);
    if (error) throw error;
  },

  signOut: async () => {
    explicitSignOut = true;
    try {
      // Local scope keeps logout deterministic even while the auth server is offline.
      const { error } = await authService.signOut();
      if (error) throw error;
      updateSession(set, null, "SIGNED_OUT");
    } finally {
      explicitSignOut = false;
    }
  },
}));
