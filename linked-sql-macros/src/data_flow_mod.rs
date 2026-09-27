#[cfg(test)]
mod tests {
    use quote::quote;

    #[test]
    fn test_data_flow() {
        let input = quote!(
            fn impl_link<Key, From, To>(
                this: SetNew<OneToMany<Key, From, To>, To::InputData>,
            ) where
                Key: Clone,
                From: Clone,
                To: Clone,
                To: Collection,
                <To::Id as CollectionId>::IdData: Clone,
            {
                let handler = use handler(self.relation);

                let insert_new_one = use pre_op(InsertOne {
                    base: handler.clone(),
                    data: AutoGenerate,
                    data: this.data,
                    links: (),
                });

                use insert_values(Bind(insert_new_one.id.clone()));

                use output(insert_new_one);
            }
        );

        let output = quote!(
            fn impl_link<Key, From, To>(this: SetNew<OneToMany<Key, From, To>, To::InputData>)
            where
                Key: Clone,
                From: Clone,
                To: Clone,
                To: Collection,
                <To::Id as CollectionId>::IdData: Clone,
            {
                InsertLinkFromClosures {
                    handler: move || self.relation,
                    pre_op: move |handler| InsertOne {
                        base: handler.clone(),
                        data: AutoGenerate,
                        data: this.data,
                        links: (),
                    },
                    insert_values: move |handler, pre_op| Bind(pre_op.id.clone()),
                    output: move |handler, pre_op| pre_op,
                }
            }
        );

        pretty_assertions::assert_eq!(input.to_string(), output.to_string());
    }
}
