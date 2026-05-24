import { Channel, invoke } from "@novonotes/webview-bridge";
import { useEffect, useRef } from "react";

export const useChannel = <T>(name: string, callback: (data: T) => void) => {
    const subscriptionIdRef = useRef<string | null>(null);

    useEffect(() => {
        const channel = new Channel(callback);
        invoke(`subscribe_${name}`, {
            channel
        }).then((result) => {
            const typed_result = result as { subscriptionId: string };
            subscriptionIdRef.current = typed_result.subscriptionId;
            console.log(`Subscribed to ${name} with subscription ID:`, typed_result.subscriptionId);
        })

        return () => {
            console.log("unmount")
            if(!subscriptionIdRef.current) return;

            invoke(`unsubscribe_${name}`, {
                subscriptionId: subscriptionIdRef.current
            });
            console.log(`Unsubscribed from ${name} with subscription ID:`, subscriptionIdRef.current);
            subscriptionIdRef.current = null;
        }
    }, [callback]);
}