import { create } from 'zustand';
import { User, Session } from '@supabase/supabase-js';
import { supabase } from '../lib/supabase';
import { useEntitlementStore } from './useEntitlementStore';

interface AuthState {
  user: User | null;
  session: Session | null;
  isInitialized: boolean;
  initialize: () => Promise<void>;
  signOut: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  session: null,
  isInitialized: false,

  initialize: async () => {
    // Check initial session
    const { data: { session } } = await supabase.auth.getSession();
    const user = session?.user ?? null;
    set({ session, user, isInitialized: true });
    
    if (user) {
        useEntitlementStore.getState().fetchEntitlements(user.id);
    } else {
        useEntitlementStore.getState().clearEntitlements();
    }

    // Listen for auth changes
    supabase.auth.onAuthStateChange((_event, newSession) => {
      const newUser = newSession?.user ?? null;
      set({ session: newSession, user: newUser });
      
      if (newUser) {
          useEntitlementStore.getState().fetchEntitlements(newUser.id);
      } else {
          useEntitlementStore.getState().clearEntitlements();
      }
    });
  },

  signOut: async () => {
    await supabase.auth.signOut();
    // Local state will be updated via onAuthStateChange listener
  }
}));
