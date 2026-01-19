// Example: Integration test app
// This demonstrates how to build test applications using the dlt-user library

use dlt_user::DltContext;
use std::thread;

fn main() {
    println!("DLT Integration Test");

    // Test 1: Single context logging
    {
        let ctx = DltContext::new("TST1", "CTX1", "Test App 1", "Context 1");
        for i in 0..5 {
            ctx.log(&format!("Test message {}", i)).unwrap();
        }
    } // Auto-cleanup

    thread::sleep(std::time::Duration::from_millis(100));

    // Test 2: Multiple contexts
    {
        let ctx1 = DltContext::new("TST2", "CTX1", "Test App 2", "Context 1");
        let ctx2 = DltContext::new("TST2", "CTX2", "Test App 2", "Context 2");

        ctx1.log("Message from context 1").unwrap();
        ctx2.log("Message from context 2").unwrap();
    }

    thread::sleep(std::time::Duration::from_millis(100));

    // Test 3: Concurrent logging
    {
        let ctx = DltContext::new("TST3", "CONC", "Test App 3", "Concurrent Context");

        let handles: Vec<_> = (0..4).map(|thread_id| {
            thread::spawn(move || {
                let local_ctx = DltContext::new("TST3", &format!("CT{}", thread_id), "Test App 3", &format!("Thread {}", thread_id));
                for i in 0..5 {
                    local_ctx.log(&format!("Thread {} msg {}", thread_id, i)).unwrap();
                }
            })
        }).collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    println!("Integration test completed");
}
