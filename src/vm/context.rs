// SPDX-License-Identifier: FSL-1.1
use crate::{
    api::{WASM_FALSE, WASM_TRUE},
    Pairs, Stack, Value,
};
use multihash::{mh, Multihash};
use multikey::{Multikey, Views};
use multisig::Multisig;
use multiutil::CodecInfo;
use std::io::Write;
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
    /// In-memory buffer to accumulate log messages from scripts
    pub log: Vec<u8>,
    /// The limiter
    pub limiter: StoreLimits,
}

impl<'a> Context<'_> {

    /// Increment the check counter and to push a FAILURE marker on the return stack
    pub fn fail(&mut self, err: &str) -> Val {
        // update the context check_count
        self.check_count += 1;
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
                self.pstack.push(v.clone()); // pushes Value::Bin(Vec<u8>)
                WASM_TRUE
            }
            None => self.fail(&format!("kvp missing key: {key}"))
        }
    }

    /// Checks the preimage proof against the hash already committed to
    pub fn check_preimage(&mut self, key: &str) -> Val {
        // look up the hash and try to decode it
        let hash = {
            match self.pairs.get(&key) {
                Some(Value::Bin(v)) => match Multihash::try_from(v.as_ref()) {
                    Ok(hash) => hash,
                    Err(e) => return self.fail(&e.to_string()),
                },
                Some(_) => return self.fail(&format!("unexpected value type associated with {key}")),
                None => return self.fail(&format!("kvp missing key: {key}")),
            }
        };

        // make sure we have at least one parameter on the stack
        if self.pstack.len() < 1 {
            return self.fail(&format!("not enough parameters on the stack for check_preimage: {}", self.pstack.len()));
        }

        // get the preimage data from the stack
        let preimage = {
            match self.pstack.top() {
                Some(Value::Bin(v)) => match mh::Builder::new_from_bytes(hash.codec(), v) {
                    Ok(builder) => match builder.try_build() {
                        Ok(hash) => hash,
                        Err(e) => return self.fail(&e.to_string()),
                    }
                    Err(e) => return self.fail(&e.to_string()),
                },
                Some(Value::Str(s)) => match mh::Builder::new_from_bytes(hash.codec(), s.as_bytes()) {
                    Ok(builder) => match builder.try_build() {
                        Ok(hash) => hash,
                        Err(e) => return self.fail(&e.to_string()),
                    }
                    Err(e) => return self.fail(&e.to_string()),
                },
                _ => return self.fail("no multihash data on stack"),
            }
        };

        // check that the hashes match
        if hash == preimage {
            // the hash check passed so pop the argument from the stack
            let _ = self.pstack.pop();
            self.succeed()
        } else {
            // the hashes don't match
            self.fail("preimage doesn't match")
        }
    }

    /// Verifies the digital signature proof with the public key already committed to
    pub fn check_signature(&mut self, key: &str) -> Val {
        // look up the hash and try to decode it
        let pubkey = {
            match self.pairs.get(key) {
                Some(Value::Bin(v)) => match Multikey::try_from(v.as_ref()) {
                    Ok(mk) => mk,
                    Err(e) => return self.fail(&e.to_string()),
                },
                Some(_) => return self.fail(&format!("unexpected value type associated with {key}")),
                None => return self.fail(&format!("no multikey associated with {key}"))
            }
        };

        // make sure we have at least two parameters on the stack
        if self.pstack.len() < 2 {
            return self.fail(&format!("not enough parameters on the stack for check_signature ({})", self.pstack.len()));
        }

        // peek at the top item and verify that it is a Multisig
        let sig = {
            match self.pstack.top() {
                Some(Value::Bin(v)) => match Multisig::try_from(v.as_ref()) {
                    Ok(sig) => sig,
                    Err(e) => return self.fail(&e.to_string()),
                },
                _ => return self.fail("no multisig on stack"),
            }
        };

        // peek at the next item down and get the message
        let msg = {
            match self.pstack.peek(1) {
                Some(Value::Bin(v)) => v,
                Some(Value::Str(s)) => s.as_bytes().to_vec(),
                _ => return self.fail("no message on stack"),
            }
        };

        // get the verify view
        let verify_view = match pubkey.verify_view() {
            Ok(v) => v,
            Err(e) => return self.fail(&e.to_string()),
        };

        // verify the signature
        match verify_view.verify(&sig, Some(msg.as_ref())) {
            Ok(_) => {
                // the signature verification worked so pop the two arguments off
                // of the stack before continuing
                self.pstack.pop();
                self.pstack.pop();
                self.succeed()
            }
            Err(e) => self.fail(&e.to_string())
        }
    }
}
