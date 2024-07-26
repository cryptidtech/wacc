// SPDX-License-Identifier: FSL-1.1
use crate::{
    api::{WASM_FALSE, WASM_TRUE},
    Pairs, Stack, Value,
};
use tracing::info;
use multihash::{mh, Multihash};
use multikey::{Multikey, Views};
use multisig::Multisig;
use multiutil::CodecInfo;
use std::{fmt, io::Write};
use wasmtime::{StoreLimits, Val};

/// Represents the application state for each instance of a WACC execution.
pub struct Context<'a>
{
    /// The key-value store
    pub pairs: &'a dyn Pairs,
    /// The stack of values
    pub pstack: &'a mut dyn Stack,
    /// The stack of return values
    pub rstack: &'a mut dyn Stack,
    /// The number of times a check_* operation has been executed
    pub check_count: usize,
    /// The top down stack index for writing into linear memory
    pub write_idx: usize,
    /// The context key-path
    pub context: String,
    /// In-memory buffer to accumulate log messages from scripts
    pub log: Vec<u8>,
    /// The limiter
    pub limiter: StoreLimits,
}

impl<'a> fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Context {{ check_count: {}, context: {} }}", self.check_count, self.context)
    }
}

impl<'a> Context<'_> {

    /// Increment the check counter and to push a FAILURE marker on the return stack
    pub fn check_fail(&mut self, err: &str) -> Val {
        // update the context check_count
        self.check_count += 1;
        // fail
        self.fail(err)
    }

    /// Increment the check counter and to push a FAILURE marker on the return stack
    pub fn fail(&mut self, err: &str) -> Val {
        // push the FAILURE onto the return stack
        self.rstack.push(Value::Failure(err.to_string()));
        // return that we failed
        WASM_FALSE
    }

    /// Push a SUCCESS marker onto the return stack
    pub fn succeed(&mut self) -> Val {
        // push the SUCCESS marker with the check count
        self.rstack.push(self.check_count.into());
        // return that we succeeded
        WASM_TRUE
    }

    /// Add a line to the log
    pub fn log(&mut self, log_line: &str) -> Val {
        // add the log line to the log
        match writeln!(&mut self.log, "{log_line}") {
            Ok(_) => WASM_TRUE,
            Err(e) => self.fail(&e.to_string()),
        }
    }

    /// Push the value associated with the key onto the parameter stack
    pub fn push(&mut self, key: &str) -> Val {
        // try to look up the key-value pair by key and push the result onto the stack
        match self.pairs.get(key) {
            Some(v) => {
                info!("push: {} -> {:?}", key, v);
                self.pstack.push(v.clone()); // pushes Value::Bin(Vec<u8>)
                WASM_TRUE
            }
            None => self.fail(&format!("kvp missing key: {key}"))
        }
    }

    /// Pop a value from the parameter stack
    pub fn pop(&mut self) -> Val {
        // make sure we have at least one parameter on the stack
        if self.pstack.len() < 1 {
            return self.fail(&format!("not enough parameters on the stack for pop ({})", self.pstack.len()));
        }

        // pop the value from the stack
        let v = self.pstack.pop();
        info!("pop: {:?}", v);
        WASM_TRUE
    }

    /// Calculate the full key given the context
    pub fn branch(&self, key: &str) -> String {
        format!("{}{}", self.context, key)
    }

    /// Verifies the top of the stack matches the value associated with the key
    #[tracing::instrument]
    pub fn check_eq(&mut self, key: &str) -> Val {
        // look up the value
        let value = {
            match self.pairs.get(key) {
                Some(v @ Value::Bin { .. }) => v,
                Some(v @ Value::Str { .. }) => v,
                Some(_) => return self.check_fail(&format!("unexpected value type associated with {key}")),
                None => return self.check_fail(&format!("no value associated with {key}"))
            }
        };

        // make sure we have at least one parameter on the stack
        if self.pstack.len() < 1 {
            return self.check_fail(&format!("not enough parameters on the stack for check_eq ({})", self.pstack.len()));
        }

        // peek at the top item
        let stack_value = {
            match self.pstack.top() {
                Some(v @ Value::Bin { .. }) => v,
                Some(v @ Value::Str { .. }) => v,
                _ => return self.check_fail("no value on stack"),
            }
        };

        // check that the values
        if value == stack_value {
            // the eq check passed so pop the argument from the stack
            let _ = self.pstack.pop();
            self.succeed()
        } else {
            // the hashes don't match
            self.check_fail("values don't match")
        }
    }

    /// Checks the preimage proof against the hash already committed to
    #[tracing::instrument]
    pub fn check_preimage(&mut self, key: &str) -> Val {
        // look up the hash and try to decode it
        let hash = {
            match self.pairs.get(&key) {
                Some(Value::Bin { hint: _, data }) => match Multihash::try_from(data.as_ref()) {
                    Ok(hash) => hash,
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                Some(_) => return self.check_fail(&format!("unexpected value type associated with {key}")),
                None => return self.check_fail(&format!("kvp missing key: {key}")),
            }
        };

        // make sure we have at least one parameter on the stack
        if self.pstack.len() < 1 {
            return self.check_fail(
                &format!("not enough parameters on the stack for check_preimage: {}", self.pstack.len())
            );
        }

        // get the preimage data from the stack
        let preimage = {
            match self.pstack.top() {
                Some(Value::Bin { hint: _, data}) => match mh::Builder::new_from_bytes(hash.codec(), data) {
                    Ok(builder) => match builder.try_build() {
                        Ok(hash) => hash,
                        Err(e) => return self.check_fail(&e.to_string()),
                    }
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                Some(Value::Str { hint: _, data }) => match mh::Builder::new_from_bytes(hash.codec(), data.as_bytes()) {
                    Ok(builder) => match builder.try_build() {
                        Ok(hash) => hash,
                        Err(e) => return self.check_fail(&e.to_string()),
                    }
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                _ => return self.check_fail("no multihash data on stack"),
            }
        };

        // check that the hashes match
        if hash == preimage {
            // the hash check passed so pop the argument from the stack
            let _ = self.pstack.pop();
            self.succeed()
        } else {
            // the hashes don't match
            self.check_fail("preimage doesn't match")
        }
    }

    /// Verifies the digital signature proof with the public key already committed to
    #[tracing::instrument]
    pub fn check_signature(&mut self, key: &str) -> Val {
        // look up the pubkey and try to decode it
        let pubkey = {
            info!(key, "pubkey: pairs::get()");
            match self.pairs.get(key) {
                Some(Value::Bin { hint:_, data }) => match Multikey::try_from(data.as_ref()) {
                    Ok(mk) => mk,
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                Some(_) => return self.check_fail(&format!("unexpected value type associated with {key}")),
                None => return self.check_fail(&format!("no multikey associated with {key}"))
            }
        };

        // make sure we have at least two parameters on the stack
        if self.pstack.len() < 2 {
            return self.check_fail(
                &format!("not enough parameters on the stack for check_signature ({})", self.pstack.len())
            );
        }

        // peek at the top item and verify that it is a Multisig
        let sig = {
            info!("sig: stack::top()");
            match self.pstack.top() {
                Some(Value::Bin { hint: _, data }) => match Multisig::try_from(data.as_ref()) {
                    Ok(sig) => sig,
                    Err(e) => return self.check_fail(&e.to_string()),
                },
                _ => return self.check_fail("no multisig on stack"),
            }
        };

        // peek at the next item down and get the message
        let msg = {
            info!("msg: stack::peek(1)");
            match self.pstack.peek(1) {
                Some(Value::Bin { hint: _, data }) => data,
                Some(Value::Str { hint: _, data }) => data.as_bytes().to_vec(),
                _ => return self.check_fail("no message on stack"),
            }
        };

        // get the verify view
        let verify_view = match pubkey.verify_view() {
            Ok(v) => v,
            Err(e) => return self.check_fail(&e.to_string()),
        };

        // verify the signature
        match verify_view.verify(&sig, Some(msg.as_ref())) {
            Ok(_) => {
                info!("verify: OK");
                // the signature verification worked so pop the two arguments off
                // of the stack before continuing
                self.pstack.pop();
                self.pstack.pop();
                self.succeed()
            }
            Err(e) => {
                info!("verify: Err({})", e.to_string());
                self.check_fail(&e.to_string())
            }
        }
    }
}
