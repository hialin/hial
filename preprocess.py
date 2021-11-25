# Script for loading & preprocessing the HotpotQA data

from pathlib import Path
from collections import Counter, OrderedDict

from utils import load_data, save_data
from utils import Querier
from kb import WikidataQueryHandler, ELQQueryHandler
from kb import PropertyStore

from fuzzywuzzy import fuzz

dataset_paths = {
    'HotpotQA': {
        'train': 'data/HotpotQA/hotpot_train_v1.1.json',
        'dev-distractor': 'data/HotpotQA/hotpot_dev_distractor_v1.json',
        'dev-distractor-sample': 'data/HotpotQA/hotpot_dev_distractor_v1.json',
        'dev-fullwiki': 'data/HotpotQA/hotpot_dev_fullwiki_v1.json',
        'test-fullwiki': 'data/HotpotQA/hotpot_test_fullwiki_v1.json'
    },
    'HotpotQA-entities-qonly': {
        'train':
            'data/HotpotQA-entities/hotpot_train_entities_v1.1_qonly.json',
        'train-graph':
            'data/HotpotQA-entities/' +
            'hotpot_train_entities_v1.1_qonly_graph_0-50.json',
        'dev-distractor':
        'data/HotpotQA-entities/hotpot_dev_distractor_entities_v1_qonly.json',
        'dev-distractor-graph':
        'data/HotpotQA-entities/' +
        'hotpot_dev_distractor_entities_v1_qonly_graph.json',
        'dev-distractor-sample':
        'data/HotpotQA-entities/' +
        'hotpot_dev_distractor_entities_v1_qonly_sample.json',
        'dev-distractor-sample-graph':
        'data/HotpotQA-entities/' +
        'hotpot_dev_distractor_entities_v1_qonly_sample_graph.json',
    },
    'HotpotQA-entities-full': {
        'train': 'data/HotpotQA-entities/hotpot_train_entities_v1.1.json',
        'dev-distractor':
        'data/HotpotQA-entities/hotpot_dev_distractor_entities_v1.json'
    }
}


def get_statistics(data):
    size = len(data)
    type_counter = Counter([question["type"] for question in data])
    level_counter = Counter([question["level"] for question in data])

    return (size, type_counter, level_counter)


def process_objects_response(response):
    results = response['results']
    properties = []
    for binding in results['bindings']:
        property_uri = binding['baseProp']['value']
        object_uri = binding['object']['value']
        object_label = binding['ooLabel']['value']
        obj = {}

        if object_uri.startswith('http://www.wikidata.org/entity/'):
            obj['uri'] = object_uri
            obj['label'] = object_label
        else:
            # it's either a literal or a blank node, just set the label
            obj['uri'] = None
            obj['label'] = object_label

        properties.append((property_uri, obj['uri'], obj['label']))

    return properties

#####
response
    results:
        bindings
            baseProp
                value
            object
                value
            ooLabel
                value

tree
    uri: ${object_uri ~= ^'http://www.wikidata.org/entity/' ? object_uri : None}
    label: $object_label

find response/results/bindings/ baseProp/value [as prop]
                                object/value [as uri]
                                ooLabel/value [as label]


def process_subjects_response(response):
    results = response['results']
    properties = []
    for binding in results['bindings']:
        property_uri = binding['baseProp']['value']
        subject_uri = binding['subject']['value']
        subject_label = binding['subjLabel']['value']
        subj = {}

        if subject_uri.startswith('http://www.wikidata.org/entity/'):
            subj['uri'] = subject_uri
            subj['label'] = subject_label
        else:
            # it's either a literal or a blank node, just set the label
            subj['uri'] = None
            subj['label'] = subject_label

        properties.append((property_uri, subj['uri'], subj['label']))

    return properties


def assign_unique_property_ids(response, wikidata_properties_ids_file):
    with open(wikidata_properties_ids_file, mode='w') as out_f:
        properties = []
        results = response['results']
        for binding in results['bindings']:
            property_uri = binding['property']['value']
            properties.append(property_uri)

        sorted_properties = sorted(properties, key=lambda x: int(
            "".join([i for i in x if i.isdigit()])))
        # print(sorted_properties)
        json_property_ids = []
        for idx in range(len(sorted_properties)):
            json_property_ids.append(
                {"property_uri": sorted_properties[idx], "id": idx})

        save_data(json_property_ids, wikidata_properties_ids_file)


def test_entity_is_class(entity_id, instance_of_list, is_subclass):
    is_class = False

    # superclasses_list = []
    # superclasses_list.append("Q2221906")  # geographic location
    # superclasses_list.append("Q811979")  # architectural structure
    # # classes_list.append("Q12280")  # bridge
    # # classes_list.append("Q39614")  # cemetery
    # superclasses_list.append("Q271669")  # landform
    # # classes_list.append("Q23397")  # lake
    # superclasses_list.append("Q55659167")  # natural watercourse
    # # classes_list.append("Q4022")  # river
    # superclasses_list.append("Q37813")  # ecosystem
    # # classes_list.append("Q4421")  # forest
    # superclasses_list.append("Q13418847")  # historical event
    # # classes_list.append("Q178561")  # battle
    # superclasses_list.append("Q781132")  # military branch
    # # classes_list.append("Q4508")  # navy
    # superclasses_list.append("Q24398318")  # religious building
    # # classes_list.append("Q16970")  # church (building)
    # superclasses_list.append("Q6881511")  # enterprise
    # # classes_list.append("Q22687")  # bank
    # superclasses_list.append("Q350604")  # armed conflict
    # # classes_list.append("Q198")  # war

    classes_list = []
    classes_list.append("Q6256")  # country
    classes_list.append("Q3624078")  # sovereign state
    classes_list.append("Q5017")  # continent
    classes_list.append("Q855697")  # subcontinent
    classes_list.append("Q3336843")  # nation within the UK
    classes_list.append("Q7930989")  # city/town
    classes_list.append("Q1093829")  # city of the US
    classes_list.append("Q1549591")  # big city
    classes_list.append("Q35657")  # state of the United States
    classes_list.append("Q515")  # city
    classes_list.append("Q5119")  # capital
    classes_list.append("Q200250")  # metropolis
    classes_list.append("Q27676416")  # city or town
    classes_list.append("Q2418896")  # part of the world
    classes_list.append("Q1637706")  # city with millions of inhabitants
    classes_list.append("Q408804")  # borrough of NYC
    classes_list.append("Q3957")  # town
    classes_list.append("Q462778")  # insular area
    classes_list.append("Q5852411")  # state of Australia
    classes_list.append("Q11828004")  # province of Canada
    classes_list.append("Q25894868")  # place type
    # designation for an administrative territorial entity
    classes_list.append("Q15617994")
    classes_list.append("Q3024240")  # historical country
    classes_list.append("Q20667921")  # type of French administrative division
    classes_list.append("Q484170")  # commune of France

    classes_list.append("Q24017414")  # second-order class
    classes_list.append("Q19361238")  # Wikidata metaclass
    classes_list.append("Q151885")  # concept
    classes_list.append("Q1437361")  # form
    classes_list.append("Q13578154")  # rank
    classes_list.append("Q427626")  # taxonomic rank
    classes_list.append("Q5633421")  # scientific journal

    classes_list.append("Q891723")  # public company
    classes_list.append("Q9174")  # religion
    classes_list.append("Q31629")  # type of sport
    classes_list.append("Q11514315")  # historical period

    classes_list.append("Q41710")  # ethnic group
    classes_list.append("Q5962346")  # classification system

    classes_list.append("Q32880")  # architectural style

    classes_list.append("6607")  # guitar
    classes_list.append("Q34379")  # musical instrument
    classes_list.append("Q128309")  # drum kit

    classes_list.append("Q34770")  # language
    classes_list.append("Q25295")  # language family

    classes_list.append("Q44148")  # male
    classes_list.append("Q467")  # woman
    classes_list.append("Q8441")  # man
    classes_list.append("Q12308941")  # male given name

    classes_list.append("Q178885")  # deity

    classes_list.append("Q28640")  # profession
    classes_list.append("Q12737077")  # occupation
    classes_list.append("Q43229")  # organization
    classes_list.append("Q17197366")  # type of organization
    # independent agency of the United States government
    classes_list.append("Q1752939")
    # independent agency of the United States government
    classes_list.append("Q1752939")
    classes_list.append("Q2122214")  # national archives

    classes_list.append("Q188451")  # music genre
    classes_list.append("Q11424")  # film
    classes_list.append("Q201658")  # film genre
    classes_list.append("Q5398426")  # television series
    classes_list.append("Q215380")  # musical group
    classes_list.append("Q106043376")  # music release type
    classes_list.append("Q18127")  # record label
    classes_list.append("Q1971694")  # game mode
    classes_list.append("Q659563")  # video game genre
    classes_list.append("Q47461344")  # written work
    classes_list.append("Q571")  # book
    classes_list.append("Q223393")  # literary genre
    classes_list.append("Q1792379")  # art genre
    classes_list.append("Q207694")  # art museum
    classes_list.append("Q27939")  # singing

    classes_list.append("Q4263830")  # literary form
    classes_list.append("Q483394")  # genre
    classes_list.append("Q7889")  # video game
    classes_list.append("Q2088357")  # musical ensemble

    classes_list.append("Q28640")  # profession
    classes_list.append("Q31629")  # type of sport
    classes_list.append("Q2312410")  # sports discipline

    classes_list.append("Q2736")  # spectator sport
    classes_list.append("Q183")  # federation
    classes_list.append("Q4611891")  # association football
    classes_list.append("Q1151733")  # baseball position
    classes_list.append("Q56019")  # military rank
    classes_list.append("Q6857706")  # military specialism
    classes_list.append("Q8473")  # military

    classes_list.append("Q66715801")  # musical profession
    classes_list.append("Q49757")  # poet
    classes_list.append("Q639669")  # musician
    classes_list.append("Q4220920")  # filmmaking occupation
    classes_list.append("Q15987302")  # legal profession
    classes_list.append("Q189533")  # academic degree
    classes_list.append("Q215380")  # musical group

    classes_list.append("Q48143")  # meningitis
    classes_list.append("Q12078")  # cancer
    classes_list.append("Q929833")  # rare disease
    classes_list.append("Q18123741")  # infectious disease
    classes_list.append("Q314676")  # notifiable disease
    classes_list.append("Q29496")  # leukemia
    classes_list.append("Q147778")  # cirrhosis

    classes_list.append("Q483247")  # phenomenon

    classes_list.append("Q12143")  # time zone

    classes_list.append("Q82799")  # name

    classes_list.append("Q8928")  # constelation
    classes_list.append("Q17444909")  # astronimical object type
    classes_list.append("Q5864")  # G-type main-sequence star
    classes_list.append("Q3235978")  # circumstelar disk

    classes_list.append("Q16334295")  # group of humans

    classes_list.append("Q1931388")  # cause of death

    classes_list.append("Q11344")  # chemical element
    classes_list.append("Q7278")  # political party

    # for cid in classes_list:
    #     if entity_id.endswith(cid):
    #         is_class = True
    if is_subclass:
        is_class = True

    for instance_of in instance_of_list:
        for cid in classes_list:
            if instance_of.endswith(cid):
                is_class = True

    return is_class


def expand_entities(item, graph_json, entities, wikidata_handler, elq_handler, property_store):
    combined_entities = {}
    combined_entities.update(entities)
    # for wikidata_id, entity in entities.items():
    #     print(f"Querying for extra entities...for {wikidata_id}")
    #     # get the text corresponding to this entity
    #     entity_text = Querier.get_results(
    #         *elq_handler.get_entity_text_query(wikidata_id))
    #     xtra_entities = Querier.get_results(
    #         *elq_handler.get_entities_from_text_query(entity_text))
    #     for ctx in xtra_entities:
    #         for entity in ctx['entities']:
    #             if 'wikidata_id' in entity and entity['wikidata_id']:
    #                 combined_entities[entity['wikidata_id']] = entity

    for wikidata_id, entity in combined_entities.items():
        print(f"Extracting subgraph for ... {wikidata_id}, {entity['entity_title']}")
        instance_of_list = []
        # generate a Wikidata subgraph
        # centered on these entities and save it
        objects_response = Querier.get_wikidata_results(
            *wikidata_handler.get_props_and_objects_query(wikidata_id))
        object_properties = process_objects_response(objects_response)

        if len(object_properties) == 0:
            print(f'Warning: no object properties discovered for {wikidata_id} {entity}')

        closest_object_properties = property_store.get_closest_object_properties(
            item['question'], object_properties)
        is_class = False
        is_subclass = False

        for p in object_properties:
            if p[0].endswith("P279"):
                is_subclass = True
                break

        for p in closest_object_properties:
            triple = {}
            triple['s'] = {
                "type": "uri",
                "value": f"http://www.wikidata.org/entity/" +
                f"{entity['wikidata_id']}",
                "label": f"{entity['entity_title']}",
                "aka": f"{augment_entity(wikidata_id, wikidata_handler)}"
            }
            triple['p'] = {
                "type": "uri",
                "value": f"{p['uri']}",
                "label": f"{p['label']}"
            }
            triple['o'] = {
                "type": "uri",
                "value": f"{p['object_uri']}",
                "label": f"{p['object_label']}"
            }
            if not p['object_uri']:
                triple['o']['value'] = None
                triple['o']['type'] = "literal"
            if triple['o']['value']:
                qid = triple['o']['value'][triple['o']['value'].rfind(
                    '/') + 1:]
                triple['o']['aka'] = augment_entity(qid, wikidata_handler)

            if triple['p']['value'].endswith("P31"):
                instance_of_list.append(triple['o']['value'])
                print(f"{triple['p']['value']}")
                print(f"{triple['o']['value']}")
                print(f"{triple['o']['label']}")

            graph_json['triples'].append(triple)

        print(f'Entity is subclass: {is_subclass}')
        if test_entity_is_class(wikidata_id, instance_of_list, is_subclass):
            continue
        subjects_response = Querier.get_wikidata_results(
            *wikidata_handler.get_props_and_subjects_query(wikidata_id))

        subject_properties = process_subjects_response(subjects_response)

        if len(subject_properties) == 0:
            continue
        closest_subject_properties = property_store.get_closest_subject_properties(
            item['question'], subject_properties)
        for p in closest_subject_properties:
            triple = {}
            triple['o'] = {
                "type": "uri",
                "value": f"http://www.wikidata.org/entity/" +
                f"{entity['wikidata_id']}",
                "label": f"{entity['entity_title']}",
                "aka": f"{augment_entity(wikidata_id, wikidata_handler)}"
            }
            triple['p'] = {
                "type": "uri",
                "value": f"{p['uri']}",
                "label": f"{p['label']}"
            }
            triple['s'] = {
                "type": "uri",
                "value": f"{p['subject_uri']}",
                "label": f"{p['subject_label']}"
            }
            if not p['subject_uri']:
                triple['s']['value'] = None
                triple['s']['type'] = "literal"
            if triple['s']['value']:
                qid = triple['s']['value'][triple['s']['value'].rfind(
                    '/') + 1:]
                triple['s']['aka'] = augment_entity(qid, wikidata_handler)

            graph_json['triples'].append(triple)


def match_entity_answer_to_nodes(item, graph_json, handler):
    if item['answer'] == 'yes':
        graph_json['answer_entity'] = handler.get_yes_entity()
    elif item['answer'] == 'no':
        graph_json['answer_entity'] = handler.get_no_entity()
    else:
        print(f'real answer {item["answer"]}')
        for triple in graph_json['triples']:
            if not graph_json['answer_entity']:
                if triple['s']['value'] and triple['s']['value'].endswith(item['answer']):
                    graph_json['answer_entity'] = triple['s']
                    break
            if not graph_json['answer_entity']:
                if triple['o']['value'] and triple['o']['value'].endswith(item['answer']):
                    graph_json['answer_entity'] = triple['o']
                    break
        if not graph_json['answer_entity']:
            graph_json['answer_entity'] = handler.get_n_a_entity()

    print(f"selected answer {graph_json['answer_entity']}")


def match_answer_to_nodes(item, graph_json, handler):
    if item['answer'] == 'yes':
        graph_json['answer_entity'] = handler.get_yes_entity()
    elif item['answer'] == 'no':
        graph_json['answer_entity'] = handler.get_no_entity()
    else:
        print(f'real answer {item["answer"]}')
        if len(item['answer_entities']) == 0:
            graph_json['answer_entity'] = handler.get_n_a_entity()
        else:
            for triple in graph_json['triples']:
                if not graph_json['answer_entity']:
                    if triple['s']['label'] == item['answer']:
                        graph_json['answer_entity'] = triple['s']
                        break
                    if 'aka' in triple['s']:
                        for aka in triple['s']['aka']:
                            if aka == item['answer']:
                                graph_json['answer_entity'] = triple['s']
                                break
                    if fuzz.ratio(triple['s']['label'], item['answer']) >= 80:
                        graph_json['answer_entity'] = triple['s']
                        break
                if not graph_json['answer_entity']:
                    if triple['o']['label'] == item['answer']:
                        graph_json['answer_entity'] = triple['o']
                        break
                    if 'aka' in triple['o']:
                        for aka in triple['o']['aka']:
                            if aka == item['answer']:
                                graph_json['answer_entity'] = triple['o']
                                break
                    # print("object ratio", triple['o']['label'], fuzz.ratio(
                    #     triple['o']['label'], item['answer']))
                    if fuzz.ratio(triple['o']['label'], item['answer']) >= 80:
                        graph_json['answer_entity'] = triple['o']
                        break
            if not graph_json['answer_entity']:
                graph_json['answer_entity'] = handler.get_n_a_entity()

    print(f"selected answer {graph_json['answer_entity']}")


def generate_answer_statistics(graph_jsons, handler):
    yeses = len([graph for graph in graph_jsons if graph['answer_entity']
                 == handler.get_yes_entity()])
    noes = len(
        [graph for graph in graph_jsons if graph['answer_entity'] == handler.get_no_entity()])
    nas = len(
        [graph for graph in graph_jsons if graph['answer_entity'] == handler.get_n_a_entity()])
    other = len([graph for graph in graph_jsons if graph['answer_entity'] != handler.get_yes_entity() and
                 graph['answer_entity'] != handler.get_no_entity() and
                 graph['answer_entity'] != handler.get_n_a_entity()])

    print(f'There are {yeses} questions with answer "yes", {noes} questions with answer "no".')
    print(f'There are {nas} questions with answer "n/a", {other} questions with a different answer.')
    print(f'{yeses + noes + nas + other} questions in total.')


if __name__ == "__main__":

    # handler = ELQQueryHandler()
    # querier = Querier()
    # question_entity_data = querier.get_results(
    #     *handler.get_entity_text_query("Q5"))

    property_store = PropertyStore.get_wikidata_property_store()

    # generate_graph_data('SimpleQuestionsWikidata-json-entities',
    #                     'dev-answerable-tiny',
    #                     'dev-answerable-graph-tiny', property_store, match_answer_entities=True, start=0, limit=0)

    # preprocess('SimpleQuestionsWikidata-json',
    #            'SimpleQuestionsWikidata-json-entities', 'dev-answerable')
    # preprocess('SimpleQuestionsWikidata-json',
    #            'SimpleQuestionsWikidata-json-entities', 'train-answerable')
    # preprocess('SimpleQuestionsWikidata-json',
    #            'SimpleQuestionsWikidata-json-entities', 'test-answerable')

    # preprocess('HotpotQA', 'HotpotQA-entities-qonly', 'train', limit=100)
    # preprocess('HotpotQA', 'HotpotQA-entities-qonly', 'dev-distractor')
    # preprocess('HotpotQA', 'HotpotQA-entities-qonly',
    #            'dev-distractor-sample', limit=10)

    # ids = set()
    # ids.add('fa504ab90e214efc97873bb76cfc8ee0')
    # generate_graph_data('SimpleQuestionsWikidata-json-entities',
    #                     'dev-answerable',
    #                     'dev-answerable-graph', property_store, match_answer_entities=True, start=0, limit=0)
    # generate_graph_data('SimpleQuestionsWikidata-json-entities',
    #                     'dev-answerable',
    #                     'dev-answerable-graph', property_store, match_answer_entities=True, start=0, limit=0)
    # generate_graph_data('SimpleQuestionsWikidata-json-entities',
    #                     'train-answerable',
    #                     'train-answerable-graph', property_store, match_answer_entities=True, start=0)
    # generate_graph_data('SimpleQuestionsWikidata-json-entities',
    #                     'test-answerable',
    #                     'test-answerable-graph', property_store, match_answer_entities=True, start=0, limit=0)

    # generate_graph_data('HotpotQA-entities-qonly',
    #                     'dev-distractor', 'dev-distractor-graph', property_store, start=0, limit=0)
    # generate_graph_data('HotpotQA-entities-qonly',
    #                     'train', 'train-graph', property_store, start=57500, limit=0)
