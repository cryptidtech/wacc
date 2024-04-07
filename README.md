[![Functional Source License 1.1](https://img.shields.io/badge/License-Functional_Source_1.1-red)][FSL]
![Rust Build](https://github.com/cryptidtech/wacc/actions/workflows/rust.yml/badge.svg)

# Web Assembly Cryptographic Constructs (WACC) VM

This Rust crate is an implementation of the [WACC VM specification][SPEC] that
is a part of the [provenance specifications][PROVSPECS]. It extends a standard
web assembly VM with the set of data manipulation and cryptographic operations
found in the WACC VM specification. Please see the specification Appendix for a
full discussion of the motivation and design principles behind the this
implementation.

## Purpose

In general, the WACC VM is designed to be a way to describe data validation 
scripts similar to the way Bitcoin scripts work when validating a Bitcoin 
transaction. WACC VM scripts operate on fields in a virtual key-value store
with strings as keys. No data is manipulated directly by the WACC scripts, they 
simply push and pop keys to the virtual stack and then execute cryptographic 
operations on the values associated with the keys on the stack and then do 
logic tests on the results.

WACC VM scripts are designed to be very simple to write. The `examples/` folder
contains examples of Rust code compiled as WACC VM scripts as well as web 
assembly text (wast) versions of scripts. Just like Bitcoin script, WACC VM 
scripts are designed to be executed in pairs with an "unlock" script executing
before the "lock" script. The "unlock" script places key values on the virtual
stack, setting up the data necessary for the "lock" script to execute the 
cryptographic operations. The goal is to be able to verify updates to the 
key-value store state.

## Use Cases

### Provenance Logs

The primary use for WACC VM scripts is to provide control over who can append 
events to a provenance log. The scripting is necessary because provenance logs 
enforce a proof "precedence" whereby verification proofs of one kind take 
precendence over proofs of other kinds. This creates the capabilities necessary
to build useful decentralized identity, intellectual property management, and
general data provenance solutions on top.

### VLADs

WACC VM Scripts play a critical role in the creation of new very long-lived 
addresses (VLADs) used to replace public keys as identifiers in loosely coupled
distributed systems. Public keys should not be used as identifiers simply 
because they are subject so compromise as well as rotation as part of a healthy
digital hygiene habit. Whenever key pairs are rotated or revoked, any
distributed system that uses public keys as identifiers suffers from a broken
link. It is the fragility of public keys that makes them terrible identifiers.

However, public keys are typically used as identifiers for two good reasons. 
First of all, they have enough entropy that they are collision-resistant
identifiers. Second, a public key is a commitment—in the cryptographic sense—to
a data validation function that can be used to validate the associated data
that it identifies.

VLADs are designed to have the same two properties as the public key but also 
not suffer from the fragility because they aren't key material. This is
accomplished by constructing a VLAD out of a large random number and the 
content address of a WACC VM validation script. The tuple creates a VLAD that 
is unique and is a cryptographic commitment to a validation function. Because 
there is no key material involved, VLADs form stable identifiers that are
exceptional at keeping distributed systems loosely coupled and connected over
long spans of time.

### VLAD Mapping Service (VMS)

One key application of VLADs—and therefore WACC VM Scripts—is using VLADs to 
map to content addresses of provenance logs. Provenance logs change over time 
and there needs to be a mutable "forward pointer" that updates to point at the 
latest entry of a provenance log in content addressable storage. The VLAD 
Mapping Service is a distributed hash table that uses VLADs as keys and CID as
the value.

This effectively works like the [IPNS DHT][IPNS] associated with [IPFS][IPFS]
and provides a way to map VLADs to the head of a provenance log. In this use
case the random value in the VLAD is a digital signature over the WACC VM
content address generated using an ephemeral key pair also used to sign the
first entry in the associated provenance log before being destroyed. This
forever binds the VLAD to the provenance log and the WACC VM script used to
validate updates to the associated CID value. This closed loop allows for
independent validation of the VLAD, the CID value, and the provenance log.

[FSL]: https://github.com/cryptidtech/provenance-log/blob/main/LICENSE.md
[SPEC]: https://github.com/cryptidtech/provenance-specifications/blob/main/specifications/wacc.md 
[PROVSPECS]: https://github.com/cryptidtech/provenance-specifications/
[IPNS]: https://docs.ipfs.tech/concepts/ipns/
[IPFS]: https://docs.ipfs.tech/
