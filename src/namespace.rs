use std::iter::Rev;
use core::slice::Items;
use std::collections::hash_map::{HashMap, Entries};
use std::collections::HashSet;

pub const NS_XMLNS_PREFIX: &'static str = "xmlns";
pub const NS_XMLNS_URI: &'static str    = "http://www.w3.org/2000/xmlns/";
pub const NS_XML_PREFIX: &'static str   = "xml";
pub const NS_XML_URI: &'static str      = "http://www.w3.org/XML/1998/namespace";
pub const NS_EMPTY_URI: &'static str    = "";

/// Denotes something which contains namespace URI mappings.
///
/// A URI mapping is a pair of type `(Option<&str>, &str)`, where the first item
/// is namespace prefix (`None` meaning default prefix) and the second item
/// is a URI mapped to the prefix.
pub trait NamespaceIterable<'a, I: Iterator<(Option<&'a str>, &'a str)>> {
    fn uri_mappings(&'a self) -> I;
}

/// Namespace is a map from prefixes to namespace URIs.
///
/// `None` prefix means no prefix (i.e. default namespace).
#[deriving(PartialEq, Clone)]
pub struct Namespace(pub HashMap<Option<String>, String>);

impl Namespace {
    /// Returns an empty namespace.
    #[inline]
    pub fn empty() -> Namespace { Namespace(HashMap::with_capacity(2)) }

    /// Checks whether this namespace is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Checks whether this namespace is essentially empty, that is, it does not contain
    /// anything but default mappings.
    pub fn is_essentially_empty(&self) -> bool {
        for (k, v) in self.0.iter() {
            match (k.as_ref().map(|k| k.as_slice()), v.as_slice()) {
                (None, u)    if u == NS_EMPTY_URI                         => {},
                (Some(p), u) if p == NS_XMLNS_PREFIX && u == NS_XMLNS_URI => {},
                (Some(p), u) if p == NS_XML_PREFIX   && u == NS_XML_URI   => {},
                _ => return false
            }
        }
        true
    }

    /// Puts a mapping into this namespace.
    ///
    /// This method does not override already existing mapping.
    ///
    /// Returns a boolean flag indicating whether the map already contained
    /// the given prefix.
    ///
    /// # Parameters
    /// * `prefix` --- namespace prefix (`None` means default namespace);
    /// * `uri`    --- namespace URI.
    ///
    /// # Return value
    /// `true` if `prefix` has been inserted successfully; `false` if the `prefix`
    /// was already present in the namespace.
    pub fn put(&mut self, prefix: Option<String>, uri: String) -> bool {
        self.0.insert(prefix, uri).is_some()
    }

    /// Queries the namespace for the given prefix.
    ///
    /// # Parameters
    /// * `prefix` --- namespace prefix (`None` means default namespace).
    ///
    /// # Return value
    /// Namespace URI corresponding to the given prefix, if it is present.
    pub fn get<'a>(&'a self, prefix: &Option<String>) -> Option<&'a str> {
        self.0.find(prefix).map(|s| s.as_slice())
    }
}

/// An iterator over mappings from prefixes to URIs in a namespace.
pub struct NamespaceMappings<'a> {
    entries: Entries<'a, Option<String>, String>
}

impl<'a> Iterator<(Option<&'a str>, &'a str)> for NamespaceMappings<'a> {
    fn next(&mut self) -> Option<(Option<&'a str>, &'a str)> {
        self.entries.next().map(|(prefix, uri)| {
            (prefix.as_ref().map(|p| p.as_slice()), uri.as_slice())
        })
    }
}

impl<'a> NamespaceIterable<'a, NamespaceMappings<'a>> for Namespace {
    fn uri_mappings(&'a self) -> NamespaceMappings<'a> {
        NamespaceMappings { entries: self.0.iter() }
    }
}

/// Namespace stack is a sequence of namespaces.
///
/// Namespace stack is used to represent cumulative namespace consisting of
/// combined namespaces from nested elements.
#[deriving(Clone, PartialEq)]
pub struct NamespaceStack(pub Vec<Namespace>);

impl NamespaceStack {
    /// Returns an empty namespace stack.
    #[inline]
    pub fn empty() -> NamespaceStack { NamespaceStack(Vec::with_capacity(2)) }

    /// Returns a namespace stack with default items in it.
    ///
    /// Default items are the following:
    ///
    /// * `xml` → `http://www.w3.org/XML/1998/namespace`;
    /// * `xmlns` → `http://www.w3.org/2000/xmlns/`.
    #[inline]
    pub fn default() -> NamespaceStack {
        let mut nst = NamespaceStack::empty();
        nst.push_empty();
        // xml namespace
        nst.put(Some(NS_XML_PREFIX.to_string()), NS_XML_URI.to_string());
        // xmlns namespace
        nst.put(Some(NS_XMLNS_PREFIX.to_string()), NS_XMLNS_URI.to_string());
        // empty namespace
        nst.put(None, NS_EMPTY_URI.to_string());
        nst
    }

    /// Adds an empty namespace to the top of this stack.
    #[inline]
    pub fn push_empty(&mut self) {
        self.0.push(Namespace::empty());
    }

    /// Removes a namespace at the top of the stack.
    ///
    /// Fails if the stack is empty.
    #[inline]
    pub fn pop(&mut self) -> Namespace {
        self.0.pop().unwrap()
    }

    /// Returns a namespace at the top of the stack, leaving the stack intact.
    ///
    /// Fails if the stack is empty.
    #[inline]
    pub fn peek<'a>(&'a mut self) -> &'a mut Namespace {
        self.0.last_mut().unwrap()
    }

    /// Puts a mapping into the topmost namespace in this stack.
    ///
    /// This method does not override a mapping in the topmost namespace if it is
    /// already present, however, it does not depend on other namespaces in the stack,
    /// so it is possible to put a mapping which is present in lower namespaces.
    ///
    /// Returns a boolean flag indicating whether the topmost namespace
    /// already contained the given prefix.
    ///
    /// # Parameters
    /// * `prefix` --- namespace prefix (`None` means default namespace);
    /// * `uri`    --- namespace URI.
    ///
    /// # Return value
    /// `true` if `prefix` has been inserted successfully; `false` if the `prefix`
    /// was already present in the namespace.
    #[inline]
    pub fn put(&mut self, prefix: Option<String>, uri: String) -> bool {
        self.0.last_mut().unwrap().put(prefix, uri)
    }

    /// Performs a search for the given prefix in the whole stack.
    ///
    /// This method walks the stack from top to bottom, querying each namespace
    /// in order for the given prefix. If none of the namespaces contains the prefix,
    /// `None` is returned.
    ///
    /// # Parameters
    /// * `prefix` --- namespace prefix (`None` means default namespace)
    #[inline]
    pub fn get<'a>(&'a self, prefix: &Option<String>) -> Option<&'a str> {
        for ns in self.0.iter().rev() {
            match ns.get(prefix) {
                None => {},
                r => return r,
            }
        }
        None
    }

    /// Combines this stack of namespaces into a single namespace.
    ///
    /// Namespaces are combined in left-to-right order, that is, rightmost namespace
    /// elements take priority over leftmost ones.
    pub fn squash(&self) -> Namespace {
        let mut result = HashMap::new();
        for ns in self.0.iter() {
            result.extend(ns.0.iter().map(|(k, v)| (k.clone(), v.to_string())));
        }
        Namespace(result)
    }
}

/// An iterator over mappings from prefixes to URIs in a namespace stack.
pub struct NamespaceStackMappings<'a> {
    namespaces: Rev<Items<'a, Namespace>>,
    current_namespace: Option<NamespaceMappings<'a>>,
    used_keys: HashSet<Option<&'a str>>
}

impl<'a> NamespaceStackMappings<'a> {
    fn to_next_namespace(&mut self) -> bool {
        self.current_namespace = self.namespaces.next().map(|ns| ns.uri_mappings());
        self.current_namespace.is_some()
    }
}

impl<'a> Iterator<(Option<&'a str>, &'a str)> for NamespaceStackMappings<'a> {
    fn next(&mut self) -> Option<(Option<&'a str>, &'a str)> {
        // If there is no current namespace and no next namespace, we're finished
        if self.current_namespace.is_none() && !self.to_next_namespace() {
            return None;
        }
        let next_item = self.current_namespace.as_mut().unwrap().next();

        match next_item {
            // There is an element in the current namespace
            Some((k, v)) => if self.used_keys.contains(&k) {
                // If the current key is used, go to the next one
                self.next()
            } else {
                // Otherwise insert the current key to the set of used keys and
                // return the mapping
                self.used_keys.insert(k);
                Some((k, v))
            },
            // Current namespace is exhausted
            None => if self.to_next_namespace() {
                // If there is next namespace, continue from it
                self.next()
            } else {
                // No next namespace, exiting
                None
            }
        }
    }
}

impl<'a> NamespaceIterable<'a, NamespaceStackMappings<'a>> for NamespaceStack {
    fn uri_mappings(&'a self) -> NamespaceStackMappings<'a> {
        NamespaceStackMappings {
            namespaces: self.0.iter().rev(),
            current_namespace: None,
            used_keys: HashSet::new()
        }
    }
}
