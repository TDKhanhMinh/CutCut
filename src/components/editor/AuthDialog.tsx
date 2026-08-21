import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { supabase } from "@/lib/supabase";

interface AuthDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

export function AuthDialog({ open, onOpenChange }: AuthDialogProps) {
    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [isSignUp, setIsSignUp] = useState(false);

    const handleAuth = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);
        setError(null);
        
        try {
            if (isSignUp) {
                const { error } = await supabase.auth.signUp({ email, password });
                if (error) throw error;
                // Sign up success, typically Supabase auto signs in if email confirmation is off
                // Or tells user to check email. For this prototype we assume auto sign-in or success.
                onOpenChange(false);
            } else {
                const { error } = await supabase.auth.signInWithPassword({ email, password });
                if (error) throw error;
                onOpenChange(false);
            }
        } catch (err: unknown) {
            setError(err instanceof Error ? err.message : "Authentication failed");
        } finally {
            setLoading(false);
        }
    };

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-md">
                <DialogHeader>
                    <DialogTitle>{isSignUp ? "Create an Account" : "Sign In"}</DialogTitle>
                    <DialogDescription>
                        {isSignUp ? "Sign up to access Cloud AI features." : "Sign in to access your Cloud AI features."}
                    </DialogDescription>
                </DialogHeader>
                
                <form onSubmit={handleAuth} className="flex flex-col gap-4 py-4">
                    {error && (
                        <div className="text-sm font-medium text-destructive bg-destructive/10 p-3 rounded-md">
                            {error}
                        </div>
                    )}
                    
                    <div className="flex flex-col gap-2">
                        <label className="text-sm font-medium" htmlFor="email">Email</label>
                        <input 
                            id="email"
                            type="email" 
                            value={email}
                            onChange={e => setEmail(e.target.value)}
                            className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                            placeholder="you@example.com"
                            required
                        />
                    </div>
                    
                    <div className="flex flex-col gap-2">
                        <label className="text-sm font-medium" htmlFor="password">Password</label>
                        <input 
                            id="password"
                            type="password" 
                            value={password}
                            onChange={e => setPassword(e.target.value)}
                            className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                            required
                        />
                    </div>
                    
                    <Button type="submit" disabled={loading} className="w-full mt-2">
                        {loading ? "Please wait..." : (isSignUp ? "Sign Up" : "Sign In")}
                    </Button>
                </form>
                
                <DialogFooter className="sm:justify-center">
                    <Button 
                        variant="link" 
                        className="text-sm text-muted-foreground"
                        onClick={() => {
                            setIsSignUp(!isSignUp);
                            setError(null);
                        }}
                    >
                        {isSignUp ? "Already have an account? Sign In" : "Need an account? Sign Up"}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
